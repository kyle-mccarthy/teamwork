use std::{borrow::Cow, convert::TryInto, str::FromStr, sync::Arc};

use async_std::sync::RwLock;
use serde::{Deserialize, Serialize};
use teamwork_schema::{Task, TaskList, TimeEntry};
use tide::{prelude::*, Body, Request, Response};

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Teamwork response missing expected header {0}")]
    MissingHeader(&'static str),
    #[error("Invalid config {0:?}")]
    ConfigError(#[from] config::ConfigError),
    #[error("IOError {0}")]
    IOError(#[from] std::io::Error),
    #[error("Teamwork API returned an error: status {0} message {1}")]
    TeamworkError(u16, &'static str, Option<serde_json::Value>),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
struct Config {
    config: Arc<RwLock<config::Config>>,
    cached: Arc<CachedConfig>,
}

struct CachedConfig {
    host: String,
    port: String,
    endpoint: String,
    api_key: Option<String>,
}

impl Config {
    fn new(config: config::Config) -> Result<Self> {
        let cached = CachedConfig {
            host: config.get_str("host")?,
            port: config.get_str("port")?,
            endpoint: config.get_str("teamwork_url")?,
            api_key: config.get_str("api_key").ok(),
        };

        Ok(Config {
            config: Arc::new(RwLock::new(config)),
            cached: Arc::new(cached),
        })
    }

    fn host(&self) -> &str {
        &self.cached.host
    }

    fn port(&self) -> &str {
        &self.cached.port
    }

    fn endpoint(&self) -> &str {
        &self.cached.endpoint
    }

    fn api_key(&self) -> Option<&str> {
        self.cached.api_key.as_deref()
    }
}

#[derive(Clone)]
struct State {
    client: Arc<surf::Client>,
    config: Config,
}

impl State {
    fn new(config: Config) -> Self {
        State {
            client: Arc::new(surf::Client::new()),
            config,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Query {
    #[serde(default = "Query::default_page")]
    page: usize,

    #[serde(rename(serialize = "pageSize"))]
    per_page: Option<usize>,

    #[serde(flatten)]
    other: std::collections::HashMap<String, serde_json::Value>,
}

impl Query {
    fn default_page() -> usize {
        1
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    page: usize,
    total_pages: usize,
    /* #[serde(rename(serialize = "perPage"))]
     * pub per_page: Option<usize>, */
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    data: Vec<T>,
    meta: Meta,
    links: Links,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Links {
    first: String,
    last: String,
    next: Option<String>,
    prev: Option<String>,
    #[serde(rename(serialize = "self"))]
    curr: String,
}

impl Links {
    fn new(url: &tide::http::Url, meta: &Meta) -> Self {
        let route = url.path();

        let params = url
            .query_pairs()
            .filter(|(param, _)| param != "page")
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>();

        let params = if params.is_empty() {
            "".to_string()
        } else {
            format!("{}&", params.join("&"))
        };

        let link = format!("{}?{}", &route, &params);

        let next = if meta.page < meta.total_pages {
            Some(format!("{}page={}", link, meta.page + 1))
        } else {
            None
        };

        let prev = if meta.page > 1 {
            Some(format!("{}page={}", link, meta.page - 1))
        } else {
            None
        };

        Links {
            first: format!("{}page=1", link),
            last: format!("{}page={}", link, meta.total_pages),
            curr: format!("{}page={}", link, meta.page),
            prev,
            next,
        }
    }
}

trait TeamworkResponse: Serialize + serde::de::DeserializeOwned {
    type Data: Serialize;

    fn data(self) -> Vec<Self::Data>;
}

/// This is the base handler responsible for proxying the data from the teamwork
/// API. The data is converted into a more standard and consistent format.
async fn base_handler<T, T2>(teamwork_route: &str, req: Request<State>) -> tide::Result
where
    T2: TeamworkResponse<Data = T>,
    T: Serialize,
{
    let auth: Cow<'_, str> = if let Some(header) = req.header("authorization") {
        Cow::Borrowed(header.as_str())
    } else {
        req.state()
            .config
            .api_key()
            .ok_or_else(|| {
                tide::Error::from_str(
                    400,
                    "Request missing authorization header and API_KEY is unnset",
                )
            })
            .map(|key| Cow::Owned(format!("Basic {}", base64::encode(format!("{}: ", &key)))))?
    };

    let query: Query = req.query()?;

    let params = req
        .url()
        .query()
        .map_or_else(|| "".into(), |q| format!("?{}", q));

    let mut response = req
        .state()
        .client
        .get(&format!(
            "{}/{}{}",
            req.state().config.endpoint(),
            teamwork_route,
            params
        ))
        .query(&query)?
        .header("Authorization", auth)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response
            .body_string()
            .await
            .map(|b| {
                serde_json::from_str::<serde_json::Value>(&b)
                    .unwrap_or_else(|_| serde_json::Value::String(b))
            })
            .ok();
        let status: u16 = response
            .status()
            .try_into()
            .expect("converting StatusCode to u16 is infallible");
        let message = response.status().canonical_reason();
        Err(Error::TeamworkError(status, message, body))?
    }

    let meta = Meta {
        page: response
            .header("X-Page")
            .map(|page| usize::from_str(page.as_str()).ok())
            .flatten()
            .unwrap_or_else(|| 1),
        total_pages: usize::from_str(
            response
                .header("X-Pages")
                .ok_or_else(|| Error::MissingHeader("X-Pages"))?
                .as_str(),
        )?,
    };

    let response: T2 = response.body_json().await?;

    let links = Links::new(req.url(), &meta);

    let mut link_header = format!("<{}>;rel=self,<{}>;rel=first", links.curr, links.first);

    if let Some(prev) = &links.prev {
        link_header.push_str(&format!(",<{}>;rel=prev", prev));
    }

    if let Some(next) = &links.next {
        link_header.push_str(&format!(",<{}>;rel=next", next));
    }

    link_header.push_str(&format!(",<{}>;rel=last", links.last));

    let response = ApiResponse {
        data: response.data(),
        links,
        meta,
    };

    let response = Response::builder(200)
        .body(Body::from_json(&response)?)
        .header("Link", &link_header);

    Ok(response.build())
}

teamwork_macros::generate_route!(all_tasks, Task, "tasks.json", "todo-items");
teamwork_macros::generate_route!(
    all_time_entries,
    TimeEntry,
    "time_entries.json",
    "time-entries"
);
teamwork_macros::generate_route!(all_task_lists, TaskList, "tasklists.json", "tasklists");

/// Intercepts errors emitted from the handler. If the error came from teamwork,
/// the status code and message are used rather than treating it as an internal
/// server error.
async fn error_handler(mut res: Response) -> tide::Result {
    // rust complains about `mutable_borrow_reservation_conflict` when
    // borrowing the error from the response while also setting the
    // response body. Doing the matching separate from setting the response
    // body, resolves the problem.
    let error = res
        .downcast_error::<Error>()
        .map(|e| match e {
            Error::TeamworkError(status, message, res_body) => {
                Some((*status, *message, res_body.clone()))
            }
            err => None,
        })
        .flatten();

    if let Some((status, message, res_body)) = error {
        res.set_status(status);

        res.set_body(serde_json::json!({
            "error": {
                "code": status,
                "message": message,
                "teamwork_response": res_body
            }

        }));
        return Ok(res);
    }

    Ok(res)
}

#[async_std::main]
async fn main() -> Result<()> {
    tide::log::start();

    let mut config = config::Config::new();

    config
        .set_default("host", "127.0.0.1")?
        .set_default("port", "3000")?
        .merge(config::File::new(".env", config::FileFormat::Toml).required(false))?
        .merge(config::Environment::new())?;

    let config = Config::new(config)?;

    let addr = format!("{}:{}", config.host(), config.port());

    let mut app = tide::with_state(State::new(config));

    app.with(tide::utils::After(error_handler));

    app.at("tasks").get(all_tasks);
    app.at("time-entries").get(all_time_entries);
    app.at("task-lists").get(all_task_lists);

    app.listen(addr).await?;

    Ok(())
}

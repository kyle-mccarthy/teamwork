use std::collections::HashMap;

use inflector::Inflector;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, token, Ident, LitStr, Result, Token, Type,
};

lazy_static::lazy_static! {
    static ref RENAMED: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert(
        "created_on", "created_at");
        m.insert(
        "last_changed_on", "updated_at");
        m
    };

}

#[derive(Debug, Default)]
struct Builder {
    structs: HashMap<String, Object>,
}

impl Builder {
    fn create_object_from_map(
        &mut self,
        name: &str,
        input_fields: &serde_json::Map<String, serde_json::Value>,
    ) {
        let fields: Vec<Field> = input_fields
            .iter()
            .map(|(old_name, value)| {
                let mut new_name = old_name.to_snake_case();

                if let Some(target_name) = RENAMED.get(new_name.as_str()) {
                    new_name = target_name.to_string();
                }
                let new_name_ident = Ident::new(&new_name, Span::call_site());

                let mut attributes: Vec<proc_macro2::TokenStream> =
                    vec![quote! { rename(deserialize = #old_name) }];

                let ty = match value {
                    serde_json::Value::String(_) => {
                        attributes.push(quote!(default));
                        attributes.push(quote! { deserialize_with =
                        "serde_with::rust::string_empty_as_none::deserialize" });
                        quote! {Option<String>}
                    }
                    serde_json::Value::Number(_) => {
                        if value.is_f64() {
                            quote! { Option<f64> }
                        } else {
                            quote! { Option<i64> }
                        }
                    }
                    serde_json::Value::Object(inner_obj) => {
                        let obj_name = old_name.to_pascal_case();

                        if !self.structs.contains_key(&obj_name) {
                            self.create_object_from_map(&obj_name, inner_obj);
                        }

                        let obj = self.structs.get(&obj_name).unwrap();

                        let obj_ident = &obj.name_ident;
                        quote! { Option<#obj_ident> }
                    }
                    serde_json::Value::Bool(_) => {
                        quote! { Option<bool> }
                    }
                    serde_json::Value::Array(arr) => {
                        if arr.len() > 0 && arr[0].is_object() {
                            let inner_obj = &arr[0]
                                .as_object()
                                .expect("is_object returned true, should unwrap to object");
                            let obj_name = old_name.to_singular().to_pascal_case();

                            if !self.structs.contains_key(&obj_name) {
                                self.create_object_from_map(&obj_name, inner_obj);
                            }

                            let obj = self.structs.get(&obj_name).unwrap();

                            let obj_ident = &obj.name_ident;

                            quote! { Option<Vec<#obj_ident>> }
                        } else {
                            quote! { Option<serde_json::Value> }
                        }
                    }
                    _ => {
                        quote! { Option<serde_json::Value> }
                    }
                };

                let attributes = quote! { #[serde(#(#attributes ,)*)] };

                let ty = Type::Verbatim(ty);

                let field = quote! {
                    #attributes
                    pub #new_name_ident: #ty,
                };

                Field {
                    old_name: old_name.clone(),
                    new_name,
                    new_name_ident,
                    ty,
                    field,
                }
            })
            .collect();

        let name = name.to_pascal_case();
        let name_ident = syn::Ident::new(&name, Span::call_site());

        let obj = Object {
            name,
            name_ident,
            fields,
        };

        self.structs.insert(obj.name.clone(), obj);
    }

    fn expand(&self) -> proc_macro2::TokenStream {
        let expanded: Vec<proc_macro2::TokenStream> = self
            .structs
            .values()
            .map(|s| {
                let name = &s.name_ident;
                let fields: Vec<&proc_macro2::TokenStream> =
                    s.fields.iter().map(|f| f.expand()).collect();

                quote! {
                    #[derive(Debug, Serialize, Deserialize)]
                    pub struct #name {
                        #(#fields)*
                    }
                }
            })
            .collect();

        quote! {
            #(#expanded)*
        }
    }
}

#[derive(Debug)]
struct Field {
    old_name: String,
    new_name: String,
    new_name_ident: Ident,
    ty: Type,
    field: proc_macro2::TokenStream,
}

impl Field {
    fn expand(&self) -> &proc_macro2::TokenStream {
        &self.field
    }
}

#[derive(Debug)]
struct Object {
    name: String,
    name_ident: Ident,
    fields: Vec<Field>,
}

#[proc_macro]
pub fn generate_schema(input: TokenStream) -> TokenStream {
    fn parse_litstr_to_json_object(
        s: &LitStr,
    ) -> Result<serde_json::Map<String, serde_json::Value>> {
        let span = s.span();
        let inner = s.value();

        serde_json::from_str::<serde_json::Value>(&inner)
            .map_err(|e| syn::Error::new(span, e.to_string()))
            .and_then(|val| {
                val.as_object()
                    .ok_or_else(|| {
                        syn::Error::new(span, "expected value to deserialize to json object")
                    })
                    .map(|v| v.clone())
            })
    }

    #[derive(Debug)]
    struct Schema {
        name: Ident,
        inner: LitStr,
        json_obj: serde_json::Map<String, serde_json::Value>,
    }

    impl Parse for Schema {
        fn parse(input: ParseStream) -> Result<Self> {
            let name = input.parse::<Ident>()?;

            input.parse::<Token![,]>()?;

            let inner = input.parse::<LitStr>()?;

            let json_obj = parse_litstr_to_json_object(&inner)?;

            Ok(Schema {
                name,
                inner,
                json_obj,
            })
        }
    }

    #[derive(Debug)]
    struct Item {
        paren_token: syn::token::Paren,
        schema: Schema,
    }

    impl Parse for Item {
        fn parse(input: ParseStream) -> Result<Self> {
            let content;

            Ok(Item {
                paren_token: syn::parenthesized!(content in input),
                schema: content.parse::<Schema>()?,
            })
        }
    }

    #[derive(Debug)]
    struct List {
        bracket_token: token::Bracket,
        items: syn::punctuated::Punctuated<Item, syn::Token![,]>,
    }

    impl List {
        fn into_schema_list(self) -> SchemaList {
            SchemaList {
                items: self.items.into_iter().map(|i| i.schema).collect(),
            }
        }
    }

    impl Parse for List {
        fn parse(input: ParseStream) -> Result<Self> {
            let content;

            Ok(List {
                bracket_token: syn::bracketed!(content in input),
                items: content.parse_terminated(Item::parse)?,
            })
        }
    }

    #[derive(Debug)]
    struct SchemaList {
        items: Vec<Schema>,
    }

    impl Parse for SchemaList {
        fn parse(input: ParseStream) -> Result<Self> {
            let lookahead = input.lookahead1();

            if lookahead.peek(Ident) {
                Ok(SchemaList {
                    items: vec![input.parse()?],
                })
            } else if lookahead.peek(token::Bracket) {
                let list = input.parse::<List>()?;
                Ok(list.into_schema_list())
            } else {
                Err(lookahead.error())
            }
        }
    }

    let i2 = input.clone();
    let schema_list = parse_macro_input!(i2 as SchemaList);

    let mut builder = Builder::default();

    schema_list
        .items
        .iter()
        .for_each(|s| builder.create_object_from_map(&s.name.to_string(), &s.json_obj));

    TokenStream::from(builder.expand())
}

#[proc_macro]
pub fn generate_route(input: TokenStream) -> TokenStream {
    struct Args {
        fn_name: Ident,
        inner_ty: Ident,
        route: LitStr,
        response_key: LitStr,
    }

    impl syn::parse::Parse for Args {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let fn_name: Ident = input.parse()?;
            let _: syn::Token![,] = input.parse()?;

            let inner_ty: Ident = input.parse()?;
            let _: syn::Token![,] = input.parse()?;
            let route: LitStr = input.parse()?;
            let _: syn::Token![,] = input.parse()?;
            let response_key: LitStr = input.parse()?;
            Ok(Args {
                fn_name,
                inner_ty,
                route,
                response_key,
            })
        }
    }

    let args = parse_macro_input!(input as Args);

    let Args {
        fn_name,
        inner_ty,
        route,
        response_key,
    } = args;

    TokenStream::from(quote! {
        async fn #fn_name(mut req: Request<State>) -> tide::Result {
            #[derive(Debug, Serialize, Deserialize)]
            struct TeamworkApiResponse {
                #[serde(rename(deserialize = #response_key))]
                data: Vec<#inner_ty>,
            }

            impl TeamworkResponse for TeamworkApiResponse {
                type Data = #inner_ty;

                fn data(self) -> Vec<Self::Data> {
                    self.data
                }
            }

            base_handler::<#inner_ty, TeamworkApiResponse>(#route, req).await
        }

    })
}

#[cfg(test)]
mod tests {}

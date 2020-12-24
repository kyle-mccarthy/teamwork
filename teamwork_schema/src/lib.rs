use serde::{Deserialize, Serialize};

teamwork_macros::generate_schema!([
    (
        Task,
        r#"{
      "id": 1,
      "boardColumn": {
        "id": 1,
        "name": "testing",
        "color": "E74C3C"
      },
      "canComplete": true,
      "comments-count": 0,
      "description": "",
      "has-reminders": false,
      "has-unread-comments": false,
      "private": 0,
      "content": "adawa",
      "order": 1,
      "project-id": 1,
      "project-name": "Project 2",
      "todo-list-id": 1,
      "todo-list-name": "Task List - Added on 03 December",
      "tasklist-private": false,
      "tasklist-isTemplate": false,
      "status": "new",
      "company-name": "MCG Company",
      "company-id": 1,
      "creator-id": 1,
      "creator-firstname": "Holly",
      "creator-lastname": "Bracken",
      "updater-id": 0,
      "updater-firstname": "",
      "updater-lastname": "",
      "completed": false,
      "start-date": "",
      "due-date-base": "",
      "due-date": "",
      "created-on": "2018-12-12T10:06:31Z",
      "last-changed-on": "2019-01-16T11:00:44Z",
      "position": 2001,
      "estimated-minutes": 0,
      "priority": "",
      "progress": 0,
      "harvest-enabled": false,
      "parentTaskId": "",
      "lockdownId": "",
      "tasklist-lockdownId": "",
      "has-dependencies": 0,
      "has-predecessors": 0,
      "hasTickets": false,
      "timeIsLogged": "0",
      "attachments-count": 0,
      "predecessors": [],
      "canEdit": true,
      "viewEstimatedTime": true,
      "creator-avatar-url": "",
      "canLogTime": true,
      "userFollowingComments": false,
      "userFollowingChanges": false,
      "DLM": 0,
      "tags": [
        {
          "id": 32661,
          "name": "On Hold",
          "color": "f4bd38",
          "projectId": 0
        }
      ],
      "parent-task": {
        "content": "ParentTask",
        "id": "17774182"
      }
    }
"#
    ),
    (
        TimeEntry,
        r#"
{
      "project-id": "1",
      "isbillable": "0",
      "tasklistId": "",
      "todo-list-name": "",
      "todo-item-name": "",
      "isbilled": "0",
      "updated-date": "2017-11-13T13:08:23Z",
      "todo-list-id": "",
      "tags": [],
      "canEdit": false,
      "taskEstimatedTime": "0",
      "company-name": "MCG Cleaning Services",
      "id": "1",
      "invoiceNo": "",
      "person-last-name": "McGill",
      "parentTaskName": "",
      "dateUserPerspective": "2014-03-30T10:10:00Z",
      "minutes": "15",
      "person-first-name": "Holly",
      "description": "",
      "ticket-id": "",
      "createdAt": "2017-11-13T13:08:23Z",
      "taskIsPrivate": "0",
      "parentTaskId": "0",
      "company-id": "1",
      "project-status": "archived",
      "person-id": "1",
      "project-name": "Website rewrite!",
      "task-tags": [],
      "taskIsSubTask": "0",
      "todo-item-id": "",
      "date": "2014-03-30T09:10:00Z",
      "has-start-time": "1",
      "hours": "1"
    }
"#
    ),
    (
        TaskList,
        r#"
    {
      "id": "1",
      "name": "task list 1",
      "description": "",
      "position": 1,
      "projectId": "1",
      "projectName": "My testing project",
      "updatedAfter": "2018-09-13T14:57:03Z",
      "private": false,
      "isTemplate": false,
      "tagged": [
        {
          "id": 32661,
          "name": "On Hold",
          "color": "f4bd38",
          "projectId": 0
        }
      ],
      "milestone-id": "",
      "pinned": false,
      "complete": false,
      "uncompleted-count": 17,
      "status": "new"
    }
  "#
    )
]);

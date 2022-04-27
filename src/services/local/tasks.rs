use anyhow::Result;
use diesel::prelude::*;

use crate::models::task::{QueryableTask, Task};
use crate::schema::tasks::dsl::*;
use crate::storage::database::DatabaseConnection;

pub fn get_tasks(list_id: String) -> Result<Vec<QueryableTask>> {
    let connection = DatabaseConnection::establish_connection();
    let results = tasks
        .filter(id_list.eq(list_id))
        .load::<QueryableTask>(&connection)?;
    Ok(results)
}

pub fn post_task(list_id: String, name: String) -> Result<()> {
    let connection = DatabaseConnection::establish_connection();
    let new_task = QueryableTask::new(name, list_id);
    diesel::insert_into(tasks)
        .values(&new_task)
        .execute(&connection)?;
    Ok(())
}

pub fn patch_task(task: &Task) -> Result<()> {
    let connection = DatabaseConnection::establish_connection();
    let task: QueryableTask = task.to_owned().into();
    diesel::update(tasks
        .filter(id_task.eq(task.id_task)))
        .set((
            id_list.eq(task.id_list),
            title.eq(task.title),
            body.eq(task.body),
            completed_on.eq(task.completed_on),
            due_date.eq(task.due_date),
            importance.eq(task.importance),
            is_reminder_on.eq(task.is_reminder_on),
            reminder_date.eq(task.reminder_date),
            status.eq(task.status),
            created_date_time.eq(task.created_date_time),
            last_modified_date_time.eq(task.last_modified_date_time),
        ))
        .execute(&connection)?;
    Ok(())
}

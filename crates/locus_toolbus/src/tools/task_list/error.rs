use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaskListError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Plan not found: {0}")]
    PlanNotFound(String),

    #[error("Missing required field for action: {0}")]
    MissingField(String),

    #[error("Invalid order: {0}")]
    InvalidOrder(String),
}

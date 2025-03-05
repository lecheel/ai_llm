#[derive(Debug)]
pub enum SseEvent {
    Text(String),
    Done,
}

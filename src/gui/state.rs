use crate::core::AuditReport;

pub(crate) enum AuditState {
    Idle,
    Loading,
    Done(AuditReport),
    Error(String),
}

pub(crate) enum AdaptState {
    Idle,
    Loading,
    Done {
        markdown: String,
        audit: AuditReport,
    },
    Error(String),
}

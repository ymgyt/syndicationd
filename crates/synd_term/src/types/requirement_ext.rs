use synd_feed::types::Requirement;

pub trait RequirementExt {
    fn display(&self) -> &'static str;
}

impl RequirementExt for Requirement {
    fn display(&self) -> &'static str {
        match self {
            Requirement::Must => "MUST",
            Requirement::Should => "SHOULD",
            Requirement::May => "MAY",
        }
    }
}

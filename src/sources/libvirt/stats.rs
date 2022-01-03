use virt::domain::{Domain, DomainState};

#[derive(Debug)]
pub struct DomainStats {
    pub domain: Domain,
    pub state: DomainState,
}

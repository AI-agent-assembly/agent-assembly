/// Agent lineage resolved from the registry for scope-chain walking.
///
/// `org_id` and `team_id` mirror the metadata keys that the lifecycle
/// service writes at registration time (keys `"org_id"` and `"team_id"`).
#[derive(Debug, Clone, Default)]
pub struct Lineage {
    pub org_id: Option<String>,
    pub team_id: Option<String>,
}

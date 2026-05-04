//! Policy scope hierarchy types (`global` / `org:<id>` / `team:<id>` / `agent:<uuid>`).
//!
//! See AAASM-219 (F92) for the design. Subsequent Sub-tasks will extend this
//! module with the `Tool(...)` variant and a scope index inside `PolicyEngine`.

use aa_core::identity::AgentId;

/// String identifier for an organisation. May be promoted to a newtype later.
pub type OrgId = String;

/// String identifier for a team. May be promoted to a newtype later.
pub type TeamId = String;

/// Hierarchical scope a policy applies to.
///
/// Resolution order is `Global → Org → Team → Agent`, with most-restrictive-wins
/// merging performed by [`crate::engine::PolicyEngine`] (wired in F93). The
/// `Tool` variant for a 5-level chain is added by AAASM-1008.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PolicyScope {
    /// Applies to every agent — the default for backward compatibility.
    Global,
    /// Applies to every agent inside the named organisation.
    Org(OrgId),
    /// Applies to every agent that belongs to the named team.
    Team(TeamId),
    /// Applies to a single specific agent.
    Agent(AgentId),
}

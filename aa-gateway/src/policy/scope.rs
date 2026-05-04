//! Policy scope hierarchy types (`global` / `org:<id>` / `team:<id>` / `agent:<uuid>`).
//!
//! See AAASM-219 (F92) for the design. Subsequent Sub-tasks will extend this
//! module with the `Tool(...)` variant and a scope index inside `PolicyEngine`.

use std::fmt;
use std::str::FromStr;

use aa_core::identity::AgentId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::policy::error::PolicyParseError;

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

impl fmt::Display for PolicyScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => f.write_str("global"),
            Self::Org(id) => write!(f, "org:{}", id),
            Self::Team(id) => write!(f, "team:{}", id),
            Self::Agent(id) => write!(f, "agent:{}", Uuid::from_bytes(*id.as_bytes())),
        }
    }
}

impl FromStr for PolicyScope {
    type Err = PolicyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = |reason: &str| PolicyParseError::InvalidScope {
            raw: s.to_owned(),
            reason: reason.to_owned(),
        };

        if s == "global" {
            return Ok(Self::Global);
        }

        let (kind, value) = match s.split_once(':') {
            Some(parts) => parts,
            None => return Err(invalid("expected `global` or `<kind>:<id>`")),
        };

        if value.is_empty() {
            return Err(invalid("identifier after ':' must not be empty"));
        }

        match kind {
            "org" => Ok(Self::Org(value.to_owned())),
            "team" => Ok(Self::Team(value.to_owned())),
            "agent" => {
                let uuid = Uuid::parse_str(value)
                    .map_err(|e| invalid(&format!("agent id is not a valid UUID: {}", e)))?;
                Ok(Self::Agent(AgentId::from_bytes(*uuid.as_bytes())))
            }
            other => Err(invalid(&format!("unknown scope kind {:?}", other))),
        }
    }
}

impl Serialize for PolicyScope {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for PolicyScope {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

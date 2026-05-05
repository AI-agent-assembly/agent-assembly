//! Test-only [`PolicyEvaluator`](crate::policy::PolicyEvaluator) implementations.
//!
//! [`DenyAllEvaluator`] and [`PermitAllEvaluator`] let downstream crates
//! write unit tests without building a real policy document or parser.
//! Gated on the `test-utils` feature.

/// Test-only policy evaluator that denies every action unconditionally.
///
/// Use `DenyAllEvaluator` in unit tests that need to assert denial paths
/// without building a real policy document.
#[cfg(all(feature = "alloc", feature = "test-utils"))]
pub struct DenyAllEvaluator;

#[cfg(all(feature = "alloc", feature = "test-utils"))]
impl crate::policy::PolicyEvaluator for DenyAllEvaluator {
    fn evaluate(
        &self,
        _ctx: &crate::AgentContext,
        _action: &crate::policy::GovernanceAction,
    ) -> crate::policy::PolicyResult {
        crate::policy::PolicyResult::Deny {
            reason: alloc::string::String::from("denied by DenyAllEvaluator"),
        }
    }

    fn load_policy(&mut self, _policy: &crate::policy::PolicyDocument) -> Result<(), crate::policy::PolicyError> {
        Ok(())
    }

    fn validate_policy(
        &self,
        _policy: &crate::policy::PolicyDocument,
    ) -> Result<(), alloc::vec::Vec<crate::policy::PolicyError>> {
        Ok(())
    }
}

/// Test-only policy evaluator that permits every action unconditionally.
///
/// Use `PermitAllEvaluator` in unit tests that need a `PolicyEvaluator`
/// but whose assertion target is not policy logic itself.
#[cfg(all(feature = "alloc", feature = "test-utils"))]
pub struct PermitAllEvaluator;

#[cfg(all(feature = "alloc", feature = "test-utils"))]
impl crate::policy::PolicyEvaluator for PermitAllEvaluator {
    fn evaluate(
        &self,
        _ctx: &crate::AgentContext,
        _action: &crate::policy::GovernanceAction,
    ) -> crate::policy::PolicyResult {
        crate::policy::PolicyResult::Allow
    }

    fn load_policy(&mut self, _policy: &crate::policy::PolicyDocument) -> Result<(), crate::policy::PolicyError> {
        Ok(())
    }

    fn validate_policy(
        &self,
        _policy: &crate::policy::PolicyDocument,
    ) -> Result<(), alloc::vec::Vec<crate::policy::PolicyError>> {
        Ok(())
    }
}

#[cfg(test)]
#[cfg(all(feature = "alloc", feature = "test-utils"))]
mod tests {
    use super::*;
    use crate::{
        identity::{AgentId, SessionId},
        policy::{GovernanceAction, PolicyEvaluator, PolicyResult},
        AgentContext,
    };

    fn make_ctx() -> AgentContext {
        AgentContext {
            agent_id: AgentId::from_bytes([0u8; 16]),
            session_id: SessionId::from_bytes([1u8; 16]),
            pid: 42,
            started_at: crate::time::Timestamp::from_nanos(0),
            metadata: alloc::collections::BTreeMap::new(),
            governance_level: crate::GovernanceLevel::default(),
            parent_agent_id: None,
            team_id: None,
            depth: 0,
            delegation_reason: None,
            spawned_by_tool: None,
        }
    }

    fn make_action() -> GovernanceAction {
        GovernanceAction::ToolCall {
            name: alloc::string::String::from("list_files"),
            args: alloc::string::String::from("{}"),
        }
    }

    #[test]
    fn permit_all_returns_allow_for_every_action() {
        let ctx = make_ctx();
        let evaluator = PermitAllEvaluator;
        assert_eq!(evaluator.evaluate(&ctx, &make_action()), PolicyResult::Allow);
        assert_eq!(
            evaluator.evaluate(
                &ctx,
                &GovernanceAction::FileAccess {
                    path: alloc::string::String::from("/tmp"),
                    mode: crate::policy::FileMode::Read,
                }
            ),
            PolicyResult::Allow
        );
    }

    #[test]
    fn deny_all_returns_deny_for_every_action() {
        let ctx = make_ctx();
        let evaluator = DenyAllEvaluator;
        let result = evaluator.evaluate(&ctx, &make_action());
        assert!(matches!(result, PolicyResult::Deny { .. }));
    }

    #[test]
    fn evaluators_are_object_safe() {
        // Compile-time check: both types can be used as trait objects.
        let _: &dyn PolicyEvaluator = &PermitAllEvaluator;
        let _: &dyn PolicyEvaluator = &DenyAllEvaluator;
    }
}

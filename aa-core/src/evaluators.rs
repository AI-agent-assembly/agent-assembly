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

    fn load_policy(
        &mut self,
        _policy: &crate::policy::PolicyDocument,
    ) -> Result<(), crate::policy::PolicyError> {
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

    fn load_policy(
        &mut self,
        _policy: &crate::policy::PolicyDocument,
    ) -> Result<(), crate::policy::PolicyError> {
        Ok(())
    }

    fn validate_policy(
        &self,
        _policy: &crate::policy::PolicyDocument,
    ) -> Result<(), alloc::vec::Vec<crate::policy::PolicyError>> {
        Ok(())
    }
}

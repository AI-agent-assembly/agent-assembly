import type { UseQueryResult } from '@tanstack/react-query'

/**
 * A query result shaped the way TanStack Query v5 actually shapes one.
 *
 * `isPending` is derived from `isLoading` unless given explicitly, because the
 * two are not interchangeable — `isLoading === isPending && isFetching` — and
 * the code under test reads **`isPending`**, not `isLoading`
 * (`certainFromQuery`, `src/lib/truthfulness/query.ts`). A hand-rolled mock
 * that sets only `isLoading` therefore fell through to the empty-payload
 * branch instead of the in-flight one, leaving the genuine in-flight
 * rendering uncovered across every consumer that used one (AAASM-5185,
 * AAASM-5252 — the single shared helper the sweep across 15 files converged
 * on, so a partial mock cannot silently select the wrong branch again).
 *
 * Extracted from `CostsPage.test.tsx`'s original local copy (AAASM-5185),
 * which remains the reference implementation this mirrors exactly.
 */
export function mockQuery<T>(p: Record<string, unknown>): UseQueryResult<T, Error> {
  return { isPending: p.isLoading === true, ...p } as unknown as UseQueryResult<T, Error>
}

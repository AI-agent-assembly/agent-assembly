import createClient from "openapi-fetch";
import type { paths } from "./generated/schema";

/** Type-safe API client generated from openapi/v1.yaml. */
export const api = createClient<paths>({
  baseUrl: import.meta.env.VITE_API_BASE_URL ?? "http://localhost:7700",
});

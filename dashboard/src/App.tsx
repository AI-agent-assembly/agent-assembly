import { useEffect, useState } from "react";
import { api } from "./api/client";

function App() {
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    api.GET("/api/v1/health").then(({ data }) => {
      if (data) {
        setStatus(data.status);
      }
    });
  }, []);

  return (
    <main>
      <h1>Agent Assembly Dashboard</h1>
      <p>Governance console — coming soon.</p>
      {status && <p>API status: {status}</p>}
    </main>
  );
}

export default App;

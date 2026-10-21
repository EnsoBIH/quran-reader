import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App, load } from "./App.tsx";
import "./index.css";

const root = document.getElementById("root");
if (!root) throw new Error("#root missing");

// Load before the first render: the text has to be on screen within 300 ms, and a
// Suspense boundary would hold the reveal back by React's fallback throttle (~300 ms).
load().then((loaded) => {
  createRoot(root).render(
    <StrictMode>
      <App loaded={loaded} />
    </StrictMode>,
  );
});

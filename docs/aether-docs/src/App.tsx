import { useEffect } from "react";
import { BrowserRouter, Navigate, Route, Routes, useLocation } from "react-router-dom";

import DocsLayout from "./components/docs/DocsLayout";
import Architecture from "./pages/docs/Architecture";
// import Configuration from "./pages/docs/Configuration";
import Introduction from "./pages/docs/Introduction";
// import Workspaces from "./pages/docs/Workspaces";

function ScrollToTop() {
  const { pathname, hash } = useLocation();

  useEffect(() => {
    if (!hash) window.scrollTo(0, 0);
  }, [pathname, hash]);

  return null;
}

export default function App() {
  return (
    <BrowserRouter>
      <ScrollToTop />
      <Routes>
        <Route path="/" element={<Navigate to="/docs/introduction" replace />} />

        <Route path="/docs" element={<DocsLayout />}>
          <Route index element={<Navigate to="introduction" replace />} />
          <Route path="introduction" element={<Introduction />} />
          {/* <Route path="configuration" element={<Configuration />} /> */}
          {/* <Route path="workspaces" element={<Workspaces />} /> */}
          <Route path="architecture" element={<Architecture />} />
          <Route path="*" element={<Navigate to="/docs/introduction" replace />} />
        </Route>

        <Route path="*" element={<Navigate to="/docs/introduction" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

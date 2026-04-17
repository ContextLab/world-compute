import React from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Routes, Route, NavLink } from "react-router-dom";
import DonorDashboard from "./pages/DonorDashboard";
import SubmitterDashboard from "./pages/SubmitterDashboard";
import GovernanceBoard from "./pages/GovernanceBoard";
import Settings from "./pages/Settings";

function Nav() {
  const linkStyle = { padding: "8px 16px", color: "#58a6ff", textDecoration: "none" };
  return (
    <nav style={{ display: "flex", gap: "4px", padding: "12px", borderBottom: "1px solid #30363d" }}>
      <NavLink to="/" style={linkStyle}>Donor</NavLink>
      <NavLink to="/submit" style={linkStyle}>Submit Job</NavLink>
      <NavLink to="/governance" style={linkStyle}>Governance</NavLink>
      <NavLink to="/settings" style={linkStyle}>Settings</NavLink>
    </nav>
  );
}

function App() {
  return (
    <BrowserRouter>
      <Nav />
      <main style={{ padding: "24px" }}>
        <Routes>
          <Route path="/" element={<DonorDashboard />} />
          <Route path="/submit" element={<SubmitterDashboard />} />
          <Route path="/governance" element={<GovernanceBoard />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </main>
    </BrowserRouter>
  );
}

const container = document.getElementById("root");
if (container) {
  createRoot(container).render(<App />);
}

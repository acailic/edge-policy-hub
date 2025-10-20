import { HashRouter, NavLink, Route, Routes, useLocation } from "react-router-dom";
import UpdateChecker from "./components/UpdateChecker";
import TenantListPage from "./pages/TenantListPage";
import TenantCreatePage from "./pages/TenantCreatePage";
import TenantEditPage from "./pages/TenantEditPage";
import PolicyBuilderPage from "./pages/PolicyBuilderPage";
import { MonitoringDashboardPage } from "./pages/MonitoringDashboardPage";
import { GlobalMonitoringPage } from "./pages/GlobalMonitoringPage";

import "./styles/monitoring.css";

function AppHeader() {
  const location = useLocation();

  if (location.pathname.includes("/policies")) {
    return (
      <header className="app-header">
        <h1>Policy Builder</h1>
      </header>
    );
  }

  if (location.pathname.includes("/tenants/new")) {
    return (
      <header className="app-header">
        <h1>Create Tenant</h1>
      </header>
    );
  }

  if (location.pathname.includes("/tenants/") && location.pathname.includes("/edit")) {
    return (
      <header className="app-header">
        <h1>Edit Tenant</h1>
      </header>
    );
  }

  if (location.pathname.includes("/monitor")) {
    return (
      <header className="app-header">
        <h1>Monitoring</h1>
      </header>
    );
  }

  return (
    <header className="app-header">
      <h1>Tenant Registry</h1>
    </header>
  );
}

function App() {
  return (
    <HashRouter>
      <UpdateChecker />
      <div className="app-shell">
        <aside className="app-sidebar">
          <div className="sidebar-header">
            <span className="brand">Edge Policy Hub</span>
          </div>
          <nav className="sidebar-nav">
            <NavLink to="/" end>
              Tenants
            </NavLink>
            <NavLink to="/tenants/new">Add Tenant</NavLink>
            <NavLink to="/monitor">Monitoring</NavLink>
            <div className="nav-section-title">Tools</div>
            <span className="nav-hint">
              Open a tenant and select "Policies" to launch the builder.
            </span>
          </nav>
        </aside>
        <main className="app-content">
          <AppHeader />
          <section className="app-main">
            <Routes>
              <Route path="/" element={<TenantListPage />} />
              <Route path="/tenants/new" element={<TenantCreatePage />} />
              <Route path="/tenants/:id/edit" element={<TenantEditPage />} />
              <Route path="/tenants/:id/policies" element={<PolicyBuilderPage />} />
              <Route path="/monitor" element={<GlobalMonitoringPage />} />
              <Route path="/tenants/:id/monitor" element={<MonitoringDashboardPage />} />
            </Routes>
          </section>
        </main>
      </div>
    </HashRouter>
  );
}

export default App;

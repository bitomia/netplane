import { BrowserRouter, Route, Routes, Navigate } from "react-router";
import { Provider } from "react-redux";

import LoginPage from "~/pages/Login";
import ProjectsPage from "~/pages/Projects";
import { store } from "~/store";
import { Toaster } from "~/components/ui/sonner";
import { useGetUserDataQuery } from "~/services/api";

function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/projects" replace />} />
      <Route path="/projects" element={<ProjectsPage />} />
      <Route path="/login" element={<LoginPage />} />
    </Routes>
  );
}

function AuthenticatedApp() {
  const { data: userData, error, isLoading } = useGetUserDataQuery();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error || !userData) {
    return <Navigate to="/login" replace />;
  }

  return (
    <>
      <AppRoutes />
      <Toaster />
    </>
  );
}

function App() {
  return (
    <Provider store={store}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/*" element={<AuthenticatedApp />} />
      </Routes>
    </Provider>
  );
}

export default function () {
  return (
    <BrowserRouter>
      <App />
    </BrowserRouter>
  );
}

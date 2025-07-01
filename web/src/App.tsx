import { BrowserRouter, Route, Routes, Navigate } from "react-router";
import { Provider } from "react-redux";

import LoginPage from "~/pages/Login";
import ProjectsPage from "~/pages/Projects";
import { store } from "~/store";
import { Toaster } from "~/components/ui/sonner";

function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/projects" replace />} />
      <Route path="/projects" element={<ProjectsPage />} />
    </Routes>
  );
}

function App() {
  return false ? (
    <Provider store={store}>
      <AppRoutes />
      <Toaster />
    </Provider>
  ) : (
    <Provider store={store}>
      <LoginPage />
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

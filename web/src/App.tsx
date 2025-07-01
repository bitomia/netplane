import { BrowserRouter, Route, Routes, Navigate } from "react-router";
import { Provider } from "react-redux";
import {
  SessionProvider,
  authConfigManager,
  useSession,
} from "@hono/auth-js/react";

import LoginPage from "~/pages/Login";
import ProjectsPage from "~/pages/Projects";
import { store } from "~/store";
import { Toaster } from "~/components/ui/sonner";

authConfigManager.setConfig({
  baseUrl: import.meta.env.VITE_GOOGLE_AUTH_URL,
  basePath: import.meta.env.VITE_GOOGLE_AUTH_PATH,
});

function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/projects" replace />} />
      <Route path="/projects" element={<ProjectsPage />} />
    </Routes>
  );
}

function App() {
  const { data: session } = useSession();
  return session ? (
    <Provider store={store}>
      <AppRoutes />
      <Toaster />
    </Provider>
  ) : (
    <LoginPage />
  );
}

export default function () {
  return (
    <SessionProvider>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </SessionProvider>
  );
}

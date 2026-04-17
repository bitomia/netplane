import { BrowserRouter, Route, Routes } from "react-router";
import { Provider } from "react-redux";

import { store } from "~/store";
import { Toaster } from "~/components/ui/sonner";
import { useGetUserDataQuery } from "~/services/api";

import DashboardPage from "~/pages/Dashboard";
import LoginPage from "~/pages/Login";

function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<DashboardPage />} />
    </Routes>
  );
}

function App() {
  const { data: userData, error, isLoading } = useGetUserDataQuery();
  return (
    <>
      {isLoading ? (
        <></>
      ) : error || !userData ? (
        <LoginPage />
      ) : (
        <>
          <AppRoutes />
          <Toaster />
        </>
      )}
    </>
  );
}

export default function () {
  return (
    <BrowserRouter>
      <Provider store={store}>
        <App />
      </Provider>
    </BrowserRouter>
  );
}

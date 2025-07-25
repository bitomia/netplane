import { NavBar } from "~/components/NavBar";
import ClientsPage from "~/pages/Clients";
import ServerStatsPage from "~/pages/StatsPage";

function DashboardPage() {
  return (
    <div className="w-screen h-screen">
      <NavBar />
      <ServerStatsPage />
      <ClientsPage />
    </div>
  );
}

export default DashboardPage;

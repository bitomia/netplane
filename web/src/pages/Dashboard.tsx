import { NavBar } from "~/components/NavBar";
import ClientsPage from "~/pages/Clients";
import ServerStatsPage from "~/pages/StatsPage";
import { useGetServerStatsQuery } from "~/services/api";

function DashboardPage() {
  const { data: stats } = useGetServerStatsQuery();

  return (
    <div className="w-screen min-h-screen flex flex-col">
      <NavBar />
      <div className="flex-1">
        <ServerStatsPage />
        <ClientsPage />
      </div>
      {stats?.version && (
        <span className="text-right py-2 pr-3 text-xs text-muted-foreground">
          {stats.version}
        </span>
      )}
    </div>
  );
}

export default DashboardPage;

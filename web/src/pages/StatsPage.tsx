import { useGetServerStatsQuery } from "~/services/api";
import { Alert } from "~/components/Alert";
import { SkeletonStatsCard, StatsCard } from "~/components/StatsCard";

function formatBytesToReadableString(bytes: number) {
  if (bytes === 0) {
    return "0 Bytes";
  }

  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

function ServerStatsPage() {
  const { data: stats, isLoading, isError } = useGetServerStatsQuery();

  return (
    <div className="max-w-5xl mx-auto px-5">
      <div className="pt-10">
        <div className="flex justify-between">
          <h1 className="text-3xl font-bold">Server</h1>
        </div>
        <div className="mt-8">
          {isLoading ? (
            <SkeletonStatsCard />
          ) : isError ? (
            <Alert>Something wrong happened. Contact support</Alert>
          ) : (
            <div className="w-full flex flex-row">
              <StatsCard
                name="Mode"
                value={
                  stats?.transport_mode.toLowerCase() == "udp"
                    ? "UDP"
                    : stats?.transport_mode.toLowerCase() == "websocket"
                      ? "WebSocket"
                      : "Unknown"
                }
              />
              <StatsCard
                name="Total Inbound"
                value={formatBytesToReadableString(stats?.in_bytes as number)}
              />
              <StatsCard
                name="Total Outbound"
                value={formatBytesToReadableString(stats?.out_bytes as number)}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default ServerStatsPage;

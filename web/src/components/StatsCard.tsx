import { Skeleton } from "~/components/ui/skeleton";

export function SkeletonStatsCard() {
  return (
    <div className="flex flex-col space-y-3 w-full my-5">
      <Skeleton className="h-[100px] w-full rounded-xl" />
    </div>
  );
}

export function StatsCard({ name, value }: { name: string; value: string }) {
  return (
    <div className="flex flex-col bg-white px-7 py-5 rounded-md mr-5">
      <span className="text-sm text-slate-400">{name}</span>
      <span className="font-bold text-lg">{value}</span>
    </div>
  );
}

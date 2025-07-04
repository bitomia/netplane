import { useState, useCallback } from "react";
import { NavBar } from "~/components/NavBar";
import { NewClientDialog } from "~/components/NewClientDialog";
import { UpdateClientDialog } from "~/components/UpdateClientDialog";
import { useGetClientsQuery } from "~/services/api";
import { Skeleton } from "~/components/ui/skeleton";
import { Alert } from "~/components/Alert";

function SkeletonClient() {
  return (
    <div className="flex flex-col space-y-3 w-full my-5">
      <Skeleton className="h-[100px] w-full rounded-xl" />
    </div>
  );
}

function ClientsPage() {
  const { data: clients, isLoading, isError } = useGetClientsQuery();
  const [openUpdate, setOpenUpdate] = useState(false);

  return (
    <div className="w-screen h-screen">
      <NavBar />
      <div className="max-w-5xl mx-auto px-5">
        <div className="py-10">
          <div className="flex justify-between">
            <h1 className="text-3xl font-bold">Clients</h1>
            <NewClientDialog />
            <UpdateClientDialog
              open={openUpdate}
              setOpenUpdate={setOpenUpdate}
            />
          </div>
          <div className="mt-8">
            {isLoading ? (
              <div>
                <SkeletonClient />
                <SkeletonClient />
                <SkeletonClient />
              </div>
            ) : isError ? (
              <Alert>Something wrong happened. Contact support</Alert>
            ) : (
              clients &&
              clients.map((c) => (
                <div
                  className="flex flex-row w-full mb-5 bg-white hover:bg-slate-200 hover:cursor-pointer  px-4 py-3 rounded-md"
                  key={c.id}
                  onClick={() => setOpenUpdate(c)}
                >
                  <div className="w-full text-slate-700">
                    <div className="flex flex-row">
                      <div className="flex flex-col w-full">
                        <span className="font-bold text-xs">SDN IP</span>
                        {c.sdn_client_ip}
                      </div>
                      <div className="flex flex-col w-full">
                        <span className="font-bold text-xs">NETMASK</span>
                        {c.netmask}
                      </div>
                      <div className="flex flex-col w-full">
                        <span className="font-bold text-xs">Authed</span>
                        {c.used ? "✅" : "⚠️"}
                      </div>
                    </div>
                    <div className="text-[10px] pb-1">{c.id}</div>
                  </div>
                  <div className="flex items-center text-slate-900">...</div>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default ClientsPage;

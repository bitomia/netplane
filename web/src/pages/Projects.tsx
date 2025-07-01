import { NavLink } from "react-router";
import { NavBar } from "~/components/NavBar";
import { NewClientDialog } from "~/components/NewClientDialog";
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

  return (
    <div className="w-screen h-screen">
      <NavBar />
      <div className="max-w-5xl mx-auto px-5">
        <div className="py-10">
          <div className="flex justify-between">
            <h1 className="text-3xl font-bold">Clients</h1>
            <NewClientDialog />
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
              clients?.map((client) => (
                <NavLink to={`/clients/${client.id}`}>
                  <div
                    key={client.id}
                    className="w-full bg-white p-4 my-5 rounded-md shadow-md hover:bg-gray-50 transition-colors hover:cursor-pointer"
                  >
                    <h2 className="text-xl font-semibold">{client.name}</h2>
                    <p className="text-sm text-gray-500">
                      Created at {new Date(client.created_at).toLocaleString()}
                    </p>
                  </div>
                </NavLink>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default ClientsPage;

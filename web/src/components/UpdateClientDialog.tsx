import { useCallback } from "react";
import { useMediaQuery } from "~/hooks/use-media-query";
import { cn } from "~/lib/utils";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import {
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
} from "~/components/ui/drawer";
import { Label } from "~/components/ui/label";
import {
  Client,
  useDeleteClientMutation,
  useGetClientsQuery,
  useSetExitNodeMutation,
  useSetUseExitNodeMutation,
} from "~/services/api";
import CopyClipboard from "../components/CopyClipboard";

function UpdateClient({
  client,
  className,
  closeForm,
}: {
  client: Client;
  closeForm: () => void;
  className?: string;
}) {
  const [deleteClient] = useDeleteClientMutation();
  const [setExitNode] = useSetExitNodeMutation();
  const [setUseExitNode] = useSetUseExitNodeMutation();
  const { data: clients } = useGetClientsQuery();

  const availableExitNodes = (clients ?? []).filter(
    (c) => c.is_exit_node && c.id !== client.id,
  );

  const onDeleteClient = useCallback(() => {
    deleteClient({ id: client.id });
    closeForm();
  }, [deleteClient, closeForm, client.id]);

  return (
    <>
      <div className={cn("grid gap-4", className)}>
        <div>
          <Label htmlFor="auth_link">Authorization Code</Label>
          <div className="mt-2">
            <p className="text-sm text-gray-500 w-full">
              Copy the following code to authenticate your client
            </p>
            <CopyClipboard className="bg-slate-100 text-slate-950 px-3 py-2 rounded w-full mt-2">
              {client.auth_link_id}
            </CopyClipboard>
          </div>
        </div>

        <div>
          <Label>Exit node</Label>
          <div className="flex items-center mt-2 gap-3">
            <input
              id="is_exit_node"
              type="checkbox"
              className="h-4 w-4"
              checked={client.is_exit_node}
              onChange={(e) =>
                setExitNode({
                  id: client.id,
                  is_exit_node: e.target.checked,
                })
              }
            />
            <label htmlFor="is_exit_node" className="text-sm">
              Serve traffic as an exit node for other clients
            </label>
          </div>
        </div>

        <div>
          <Label htmlFor="use_exit_node">Use exit node</Label>
          <div className="mt-2">
            <select
              id="use_exit_node"
              className="w-full bg-background border rounded-md px-2 py-2 text-sm"
              value={client.exit_node_id ?? ""}
              onChange={(e) =>
                setUseExitNode({
                  id: client.id,
                  exit_node_id: e.target.value === "" ? null : e.target.value,
                })
              }
            >
              <option value="">None (direct internet)</option>
              {availableExitNodes.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.sdn_client_ip}
                </option>
              ))}
            </select>
            <p className="text-xs text-muted-foreground mt-1">
              Route this client's non-SDN traffic through another peer.
            </p>
          </div>
        </div>

        <div>
          <Label>Delete client</Label>
          <div className="flex mt-2">
            <p className="text-sm text-gray-500 col-span-2">
              Are you sure you want to delete this client? All of your data will
              be permanently removed. This action cannot be undone.
            </p>
            <Button variant="destructive" onClick={onDeleteClient}>
              Delete
            </Button>
          </div>
        </div>
      </div>
    </>
  );
}

export function UpdateClientDialog({
  open,
  setOpen,
}: {
  open: Client | null | false;
  setOpen: (c: Client | null | false) => void;
}) {
  const isDesktop = useMediaQuery("(min-width: 768px)");
  const closeDialog = useCallback(() => setOpen(null), [setOpen]);

  if (!open) {
    return null;
  }

  if (isDesktop) {
    return (
      <Dialog open={!!open} onOpenChange={(o) => !o && closeDialog()}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>Update Client</DialogTitle>
            <DialogDescription>Update or delete a client</DialogDescription>
          </DialogHeader>
          <UpdateClient client={open} closeForm={closeDialog} />
          <DialogFooter className="px-0">
            <DialogClose asChild>
              <Button onClick={closeDialog}>Close</Button>
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Drawer open={!!open} onOpenChange={(o) => !o && closeDialog()}>
      <DrawerContent>
        <DrawerHeader className="text-left">
          <DrawerTitle>Update Client</DrawerTitle>
          <DrawerDescription>Update or delete a client</DrawerDescription>
        </DrawerHeader>
        <UpdateClient
          className="px-4"
          client={open}
          closeForm={closeDialog}
        />
        <DrawerFooter className="px-4">
          <DrawerClose asChild>
            <Button onClick={closeDialog}>Close</Button>
          </DrawerClose>
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  );
}

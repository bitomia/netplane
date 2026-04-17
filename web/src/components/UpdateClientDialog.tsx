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
import { useDeleteClientMutation } from "~/services/api";
import CopyClipboard from "../components/CopyClipboard";

function UpdateClient({
  id,
  authLink,
  className,
  closeForm,
}: {
  id: string;
  authLink: string;
  closeForm: () => void;
  className?: string;
}) {
  const [deleteClient] = useDeleteClientMutation();
  const onDeleteClient = useCallback(() => {
    deleteClient({ id });
    closeForm();
  }, [deleteClient, closeForm, id]);

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
              {authLink}
            </CopyClipboard>
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

export function UpdateClientDialog({ open, setOpen }) {
  const isDesktop = useMediaQuery("(min-width: 768px)");
  const closeDialog = useCallback(() => setOpen(null), [setOpen]);

  if (isDesktop) {
    return (
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>Update Client</DialogTitle>
            <DialogDescription>Update or delete a client</DialogDescription>
          </DialogHeader>
          <UpdateClient
            id={open?.id}
            authLink={open?.auth_link_id}
            closeForm={closeDialog}
          />
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
    <Drawer open={open} onOpenChange={setOpen}>
      <DrawerContent>
        <DrawerHeader className="text-left">
          <DrawerTitle>Update Client</DrawerTitle>
          <DrawerDescription>Update or delete a client</DrawerDescription>
        </DrawerHeader>
        <UpdateClient
          className="px-4"
          id={open?.id}
          authLink={open?.auth_link_id}
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

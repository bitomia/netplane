import { useCallback, useState } from "react";
import { useMediaQuery } from "~/hooks/use-media-query";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "~/components/ui/dialog";
import {
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
  DrawerTrigger,
} from "~/components/ui/drawer";
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
  const [deleteClient, { status, isLoading }] = useDeleteClientMutation();
  const onDeleteClient = useCallback(() => {
    deleteClient({ id });
    closeForm();
  }, [deleteClient, closeForm, id]);

  return (
    <>
      <div className="grid gap-2">
        <Label htmlFor="auth_link">Auth Link</Label>
        <div className="mt-2">
          <p className="text-sm text-gray-500 w-full">
            Copy the following link to auth your client
          </p>
          <CopyClipboard className="bg-slate-100 text-slate-950 px-3 py-2 rounded w-full mt-2">
            {authLink}
          </CopyClipboard>
        </div>
      </div>

      <Label>Delete client</Label>

      <p className="text-sm text-gray-500">
        Are you sure you want to delete this client? All of your data will be
        permanently removed. This action cannot be undone.
      </p>
      <button
        type="button"
        className="shrink rounded-md bg-red-600 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-red-500 sm:ml-3 sm:w-auto"
        onClick={onDeleteClient}
      >
        Delete
      </button>

      <button
        type="button"
        className="mt-3 inline-flex w-full justify-center rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 ring-1 shadow-xs ring-gray-300 ring-inset hover:bg-gray-50 sm:mt-0 sm:w-auto"
        onClick={() => closeForm()}
      >
        Close
      </button>
    </>
  );
}

export function UpdateClientDialog({ open, setOpen }) {
  const isDesktop = useMediaQuery("(min-width: 768px)");
  const closeDialog = useCallback(() => setOpen(null), [setOpen]);

  if (isDesktop) {
    return (
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-[425px]">
          <DialogHeader>
            <DialogTitle>Update Client</DialogTitle>
            <DialogDescription>Update or delete a client</DialogDescription>
          </DialogHeader>
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
          id={open?.id}
          authLink={open?.auth_link_id}
          className="px-4"
          closeForm={closeDialog}
        />
        <DrawerFooter className="pt-2">
          <DrawerClose asChild>
            <Button variant="outline">Cancel</Button>
          </DrawerClose>
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  );
}

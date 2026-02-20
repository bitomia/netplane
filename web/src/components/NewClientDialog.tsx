import { FormEvent, useCallback, useState } from "react";
import { cn } from "~/lib/utils";
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
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Plus } from "lucide-react";
import { useCreateClientMutation } from "~/services/api";

function NewClientForm({
  className,
  closeForm,
}: React.ComponentProps<"form"> & { closeForm: () => void }) {
  const [createClient, { isLoading }] = useCreateClientMutation();
  const [error, setError] = useState<string | null>(null);

  const onCreateClient = useCallback(
    async (e: FormEvent<HTMLFormElement>) => {
      e.preventDefault();
      setError(null);
      const formData = new FormData(e.currentTarget);
      const { sdn_client_ip, netmask } = Object.fromEntries(
        formData.entries(),
      ) as {
        sdn_client_ip: string;
        netmask: string;
      };
      try {
        await createClient({ sdn_client_ip, netmask: "255.255.255.0" }).unwrap();
        closeForm();
      } catch {
        setError("A client with this IP address already exists.");
      }
    },
    [createClient, closeForm],
  );

  return (
    <form
      className={cn("grid items-start gap-4", className)}
      onSubmit={onCreateClient}
    >
      <div className="grid gap-2">
        <Label htmlFor="sdn_client_ip">SDN IP Address</Label>
        <Input
          type="text"
          id="sdn_client_ip"
          name="sdn_client_ip"
          placeholder="10.0.0.1"
          required
        />
      </div>
      {error && (
        <p className="text-sm text-destructive">{error}</p>
      )}
      <Button type="submit" disabled={isLoading}>Create client</Button>
    </form>
  );
}

export function NewClientDialog() {
  const [open, setOpen] = useState(false);
  const isDesktop = useMediaQuery("(min-width: 768px)");
  const closeForm = useCallback(() => setOpen(false), [setOpen]);

  if (isDesktop) {
    return (
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <Button variant="outline" size="icon" className="scale-125">
            <Plus />
          </Button>
        </DialogTrigger>
        <DialogContent className="sm:max-w-[425px]">
          <DialogHeader>
            <DialogTitle>Create Client</DialogTitle>
            <DialogDescription>
              Add a new client to the network
            </DialogDescription>
          </DialogHeader>
          <NewClientForm closeForm={closeForm} />
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Drawer open={open} onOpenChange={setOpen}>
      <DrawerTrigger asChild>
        <Button variant="outline" size="icon">
          <Plus />
        </Button>
      </DrawerTrigger>
      <DrawerContent>
        <DrawerHeader className="text-left">
          <DrawerTitle>Create Client</DrawerTitle>
          <DrawerDescription>Add a new client to the network</DrawerDescription>
        </DrawerHeader>
        <NewClientForm className="px-4" closeForm={closeForm} />
        <DrawerFooter className="pt-2">
          <DrawerClose asChild>
            <Button variant="outline">Cancel</Button>
          </DrawerClose>
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  );
}

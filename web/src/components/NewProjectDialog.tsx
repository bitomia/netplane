import { FormEvent, useCallback, useState } from "react";
import { cn } from "~/lib/utils";
import { useMediaQuery } from "~/hooks/use-media-query";
import { Button } from "~/components/ui/button"
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
} from "~/components/ui/drawer"
import { Input } from "~/components/ui/input"
import { Label } from "~/components/ui/label"
import { Plus } from "lucide-react";
import { useCreateProjectMutation } from "~/services/api";

function NewProjectForm({ className, closeForm }: React.ComponentProps<"form"> & { closeForm: () => void }) {
  const [createProject, { status, isLoading }] = useCreateProjectMutation();
  const onCreateProject = useCallback((e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const { name } = Object.fromEntries(formData.entries()) as { name: string }
    createProject({ name });
    closeForm();
  }, [createProject]);

  return (
    <form className={cn("grid items-start gap-4", className)} onSubmit={onCreateProject}>
      <div className="grid gap-2">
        <Label htmlFor="name">Project name</Label>
        <Input type="text" id="name" name="name" placeholder="My first project" required />
      </div>
      <Button type="submit">Create project</Button>
    </form>
  )
}

export function NewProjectDialog() {
  const [open, setOpen] = useState(false)
  const isDesktop = useMediaQuery("(min-width: 768px)")
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
            <DialogTitle>Create Project</DialogTitle>
            <DialogDescription>
              Create a new project.
            </DialogDescription>
          </DialogHeader>
          <NewProjectForm closeForm={closeForm} />
        </DialogContent>
      </Dialog>
    )
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
          <DrawerTitle>Create Project</DrawerTitle>
          <DrawerDescription>
            Create a new project.
          </DrawerDescription>
        </DrawerHeader>
        <NewProjectForm className="px-4" closeForm={closeForm} />
        <DrawerFooter className="pt-2">
          <DrawerClose asChild>
            <Button variant="outline">Cancel</Button>
          </DrawerClose>
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  )
}


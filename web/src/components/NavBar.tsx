import {
  BreadcrumbPage,
  BreadcrumbItem,
  BreadcrumbList,
  Breadcrumb,
  BreadcrumbSeparator,
  BreadcrumbLink,
} from "~/components/ui/breadcrumb";
import { Avatar, AvatarFallback, AvatarImage } from "~/components/ui/avatar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import { useMemo } from "react";
import { useLocation } from "react-router";
import { NavLink } from "react-router";
import { useGetClientsQuery } from "~/services/api";
import logo from "~/assets/small_icon.svg";

function PageBreadcrumb() {
  const location = useLocation();
  const locationPath = useMemo(
    () => location.pathname.split("/"),
    [location.pathname],
  );
  const { data: projects, isLoading, isError } = useGetClientsQuery();
  const path = useMemo(() => {
    if (isLoading || isError) return [];
    return locationPath?.map((p, idx) => {
      const project = projects?.find((proj) => proj.id === p);
      return {
        path: locationPath.slice(0, idx + 1).join("/"),
        name: project ? project.name : p,
        isLatest: idx === locationPath.length - 1,
      };
    });
  }, [locationPath, projects, isLoading, isError]);

  return (
    <Breadcrumb>
      <BreadcrumbList>
        <BreadcrumbItem>
          <BreadcrumbLink href="/">
            <img src={logo} alt="logo" className="max-w-8 pb-1" />
          </BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        {path?.map((p, idx) => (
          <>
            <BreadcrumbItem key={`bc-${idx}`}>
              {p.isLatest ? (
                <BreadcrumbPage className="capitalize">
                  {" "}
                  {p.name}
                </BreadcrumbPage>
              ) : (
                <BreadcrumbLink className="capitalize" asChild={true}>
                  <NavLink to={p.path}>{p.name}</NavLink>
                </BreadcrumbLink>
              )}
            </BreadcrumbItem>
            {idx > 0 && idx < path.length - 1 && <BreadcrumbSeparator />}
          </>
        ))}
      </BreadcrumbList>
    </Breadcrumb>
  );
}

function AvatarMenu() {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Avatar>
          <AvatarImage src={undefined} />
          <AvatarFallback>Username</AvatarFallback>
        </Avatar>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-56">
        <DropdownMenuLabel>
          username
          <br />
          email
        </DropdownMenuLabel>
        <DropdownMenuItem>Account Settings</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={() => console.log("logout")}>
          Log out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function NavBar({ children }) {
  return (
    <div className="w-full bg-white border-b border-zinc-200">
      <div className="max-w-5xl mx-auto">
        <div className="w-full py-6 flex justify-between px-5">
          <PageBreadcrumb />
          <div className="flex">
            <AvatarMenu />
          </div>
        </div>
      </div>
      {children}
    </div>
  );
}

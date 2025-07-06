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
import { useCallback, useMemo } from "react";
import { useNavigate, useLocation } from "react-router";
import { NavLink } from "react-router";
import logo from "~/assets/small_icon.svg";
import { useGetUserDataQuery } from "~/services/api";

function PageBreadcrumb() {
  const location = useLocation();
  const locationPath = useMemo(
    () => location.pathname.split("/"),
    [location.pathname],
  );

  const path = useMemo(() => {
    return locationPath?.map((p, idx) => {
      return {
        path: locationPath.slice(0, idx + 1).join("/"),
        name: p,
        isLatest: idx === locationPath.length - 1,
      };
    });
  }, [locationPath]);

  return (
    <Breadcrumb>
      <BreadcrumbList>
        <BreadcrumbItem>
          <BreadcrumbLink href="/">
            <img src={logo} alt="logo" className="max-w-5" />
          </BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        {path?.map((p, idx) => (
          <>
            <BreadcrumbItem key={`bc-${idx}`}>
              {p.isLatest ? (
                <BreadcrumbPage className="capitalize">{p.name}</BreadcrumbPage>
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
  const { data: userData } = useGetUserDataQuery();
  const navigate = useNavigate();

  const onLogout = useCallback(async () => {
    await fetch(`${import.meta.env.VITE_API_URL}logout`, {
      credentials: "include",
    });
    window.location.reload();
    navigate("/");
  }, [navigate]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Avatar>
          <AvatarImage src={undefined} />
          <AvatarFallback>
            {userData?.email?.charAt(0).toUpperCase()}
          </AvatarFallback>
        </Avatar>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-56">
        <DropdownMenuLabel>{userData?.email}</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={() => onLogout()}>Log out</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function NavBar() {
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
    </div>
  );
}

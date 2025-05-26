import { type RouteConfig, route } from "@react-router/dev/routes";

export default [
  route("/", "./pages/home.tsx"),
  // pattern ^           ^ module file
] satisfies RouteConfig;

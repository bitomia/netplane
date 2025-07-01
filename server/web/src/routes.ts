import { type RouteConfig, route } from "@react-router/dev/routes";

export default [
  route("/", "./pages/Home.jsx"),
  // pattern ^           ^ module file
] satisfies RouteConfig;

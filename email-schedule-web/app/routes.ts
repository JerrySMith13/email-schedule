import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
    route("about", "routes/about.tsx"),
    route("redirect-auth", "routes/redirect-auth.tsx"),
    index("routes/home.tsx")] satisfies RouteConfig;

import { Link } from "react-router";
import type { Route } from "./+types/home";

export function meta({}: Route.MetaArgs) {
  return [
    { title: "Sign Up" },
    { name: "description", content: "Sign up for RHS email notifications" },
  ];
}

export default function Home() {

  

  return(
  <>
    <h1>Sign Up</h1>
    <p>Sign up for RHS email notifications</p>
    <Link to="https://app.blackbaud.com/oauth/authorize?
      client_id=a73435c7-f62a-4101-86a5-0792c0c32ef2
      &response_type=code
      &redirect_uri=http://localhost:3000/redirect-auth"/>
     
  
  </>
  );
}

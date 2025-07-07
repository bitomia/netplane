import { LoginForm } from "~/components/login-form";

function LoginPage() {
  return (
    <div className="w-screen h-screen flex justify-center items-center">
      <LoginForm className="w-96" />
    </div>
  );
}

export default LoginPage;

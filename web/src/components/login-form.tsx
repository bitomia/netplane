import { FormEvent, useCallback, useState } from "react";
import { cn } from "~/lib/utils";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Button } from "~/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "~/components/ui/card";
import logo from "~/assets/logo.svg";
import { signIn } from "@hono/auth-js/react";

export function LoginForm({
  className,
  ...props
}: React.ComponentProps<"div">) {
  const onSignIn = useCallback((e: FormEvent<HTMLFormElement>) => {});

  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card>
        <CardHeader className="text-center">
          <img src={logo} alt="logo" className="max-w-24 pb-2 mx-auto" />
          <CardTitle className="text-xl">Welcome back</CardTitle>
          <CardDescription>Login with your credentials</CardDescription>
        </CardHeader>
        <CardContent>
          <form className={"grid items-start gap-4"} onSubmit={onSignIn}>
            <div className="grid gap-3">
              <Input
                type="text"
                id="username"
                name="username"
                placeholder="Email or username"
                required
              />
              <Input
                type="password"
                id="password"
                name="password"
                placeholder="password"
                required
              />
              <Button type="submit">Sign In</Button>
            </div>
          </form>
        </CardContent>
      </Card>
      <div className="text-muted-foreground *:[a]:hover:text-primary text-center text-xs text-balance *:[a]:underline *:[a]:underline-offset-4">
        By clicking continue, you agree to our{" "}
        <a href="https://keenchat.com/terms">Terms of Service</a> and{" "}
        <a href="https://keenchat.com/privacy">Privacy Policy</a>.
      </div>
    </div>
  );
}

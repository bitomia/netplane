import { FormEvent, useCallback, useState } from "react";
import { useNavigate } from "react-router";
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
import { useLoginMutation } from "~/services/api";

export function LoginForm({
    className,
    ...props
}: React.ComponentProps<"div">) {
    const [login, { isLoading, error }] = useLoginMutation();
    const [formData, setFormData] = useState({ email: "", password: "" });
    const navigate = useNavigate();

    const onSignIn = useCallback(
        async (e: FormEvent<HTMLFormElement>) => {
            e.preventDefault();
            try {
                const ret = await login(formData).unwrap();
                if (ret?.success) {
                    window.location.reload();
                }
            } catch (err) {
                console.error("Login failed:", err);
            }
        },
        [login, formData, navigate],
    );

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
                                type="email"
                                id="email"
                                name="email"
                                placeholder="Email"
                                value={formData.email}
                                onChange={(e) =>
                                    setFormData({ ...formData, email: e.target.value })
                                }
                                required
                            />
                            <Input
                                type="password"
                                id="password"
                                name="password"
                                placeholder="Password"
                                value={formData.password}
                                onChange={(e) =>
                                    setFormData({ ...formData, password: e.target.value })
                                }
                                required
                            />
                            {error && (
                                <div className="text-red-700 text-sm px-5 py-3 bg-red-50 font-medium">
                                    {"data" in error ? error.data : "Login failed"}
                                </div>
                            )}
                            <Button type="submit" disabled={isLoading}>
                                {isLoading ? "Signing In..." : "Sign In"}
                            </Button>
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

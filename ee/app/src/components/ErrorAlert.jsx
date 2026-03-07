import { AlertCircle } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

export default function ErrorAlert({ children }) {
  return (
    <Alert variant="destructive" className="max-w-md m-8">
      <AlertCircle className="h-4 w-4 stroke-red-600 dark:stroke-red-400" />
      <AlertTitle className="text-red-600 dark:text-red-400">Error</AlertTitle>
      <AlertDescription className="text-red-600 dark:text-red-400">
        {children}
      </AlertDescription>
    </Alert>
  );
}

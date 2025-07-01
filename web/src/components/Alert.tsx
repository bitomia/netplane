import { AlertCircle } from "lucide-react"
import {
  Alert as AlertDefault,
  AlertDescription,
  AlertTitle,
} from "~/components/ui/alert"

export function Alert({ children }) {
  return (
    <AlertDefault variant="destructive">
      <AlertCircle className="h-4 w-4" />
      <AlertTitle>Error</AlertTitle>
      <AlertDescription>
        {children}
      </AlertDescription>
    </AlertDefault>
  )
}


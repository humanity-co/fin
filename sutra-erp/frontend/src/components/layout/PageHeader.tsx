import { type ReactNode } from "react";
import { Link } from "react-router-dom";
import { ChevronRight, Home } from "lucide-react";
interface BreadcrumbItem {
  label: string;
  href?: string;
}

interface PageHeaderProps {
  title: string;
  description?: string;
  breadcrumbs?: BreadcrumbItem[];
  actions?: ReactNode;
}

export function PageHeader({
  title,
  description,
  breadcrumbs,
  actions,
}: PageHeaderProps) {
  return (
    <div className="mb-8 flex flex-col gap-3 relative">
      {/* Breadcrumbs */}
      {breadcrumbs && breadcrumbs.length > 0 && (
        <nav className="mb-3 flex items-center gap-1.5 text-xs font-medium text-slate-500 relative z-10">
          <Link to="/" className="hover:text-primary transition-colors p-1 rounded-md">
            <Home className="h-3.5 w-3.5" />
          </Link>
          {breadcrumbs.map((crumb, i) => (
            <span key={i} className="flex items-center gap-1.5">
              <ChevronRight className="h-3.5 w-3.5 text-slate-400" />
              {crumb.href ? (
                <Link
                  to={crumb.href}
                  className="hover:text-primary transition-colors px-1 py-0.5 rounded-md hover:bg-slate-100"
                >
                  {crumb.label}
                </Link>
              ) : (
                <span className="font-semibold text-slate-700 px-1 py-0.5">
                  {crumb.label}
                </span>
              )}
            </span>
          ))}
        </nav>
      )}

      {/* Title row */}
      <div className="flex items-start justify-between gap-4 relative z-10">
        <div>
          <h1 className="text-3xl font-extrabold tracking-tight text-slate-900">
            {title}
          </h1>
          {description && (
            <p className="mt-1.5 text-sm text-slate-500 font-medium">{description}</p>
          )}
        </div>
        {actions && <div className="flex shrink-0 items-center gap-3">{actions}</div>}
      </div>
    </div>
  );
}

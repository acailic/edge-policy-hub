import { Shield, Sparkles, Waypoints } from "lucide-react";

import {
  SAMPLE_POLICIES,
  getPolicyDescription,
} from "../utils/sample-policies";

interface PolicyTemplateSelectorProps {
  onSelectTemplate: (dsl: string, description: string) => void;
}

const TEMPLATE_METADATA = [
  {
    key: "data_residency",
    title: "Data Residency (EU-only)",
    icon: <Shield />,
  },
  {
    key: "cost_guardrail",
    title: "Cost Guardrail (Bandwidth)",
    icon: <Waypoints />,
  },
  {
    key: "multi_tenant_separation",
    title: "Multi-Tenant Separation",
    icon: <Sparkles />,
  },
] as const;

export function PolicyTemplateSelector({
  onSelectTemplate,
}: PolicyTemplateSelectorProps) {
  const combinedTemplate = Object.values(SAMPLE_POLICIES).join("\n\n");

  return (
    <div className="policy-template-selector">
      <div className="template-grid">
        {TEMPLATE_METADATA.map((template) => (
          <article key={template.key} className="template-card">
            <header>
              <span className="template-icon">{template.icon}</span>
              <h3>{template.title}</h3>
            </header>
            <p>{getPolicyDescription(template.key)}</p>
            <pre className="template-preview">
              {SAMPLE_POLICIES[template.key].split("\n").slice(0, 3).join("\n")}
              {"\n…"}
            </pre>
            <button
              type="button"
              onClick={() =>
                onSelectTemplate(
                  SAMPLE_POLICIES[template.key],
                  template.title,
                )
              }
            >
              Use Template
            </button>
          </article>
        ))}

        <article className="template-card">
          <header>
            <span className="template-icon">
              <Sparkles />
            </span>
            <h3>Combined Guardrails</h3>
          </header>
          <p>
            Start with all standard guardrails applied together: data
            residency, cost protection, and tenant isolation.
          </p>
          <pre className="template-preview">
            {combinedTemplate.split("\n").slice(0, 6).join("\n")}
            {"\n…"}
          </pre>
          <button
            type="button"
            onClick={() =>
              onSelectTemplate(
                combinedTemplate,
                "Combined Guardrails (baseline)",
              )
            }
          >
            Use Template
          </button>
        </article>
      </div>

      <div className="template-footer">
        <button
          type="button"
          onClick={() =>
            onSelectTemplate(
              "# New Policy\n# Describe the intent of your policy here\n",
              "Blank policy template",
            )
          }
        >
          Start from Scratch
        </button>
      </div>
    </div>
  );
}

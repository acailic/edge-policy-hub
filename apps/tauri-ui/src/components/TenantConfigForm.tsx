import { Controller, FieldErrors, Control, UseFormRegister } from "react-hook-form";

const DATA_RESIDENCY_OPTIONS = ["EU", "US", "APAC"] as const;

export interface TenantConfigFormValues {
  quotas: {
    message_limit: number;
    bandwidth_limit_gb: number;
  };
  features: {
    data_residency: string[];
    pii_redaction: boolean;
  };
}

interface TenantConfigFormProps<FormValues extends TenantConfigFormValues> {
  register: UseFormRegister<FormValues>;
  control: Control<FormValues>;
  errors: FieldErrors<FormValues>;
}

function TenantConfigForm<FormValues extends TenantConfigFormValues>({
  register,
  control,
  errors,
}: TenantConfigFormProps<FormValues>) {
  return (
    <div className="form-grid">
      <fieldset className="form-section">
        <legend>Quotas</legend>
        <p className="helper-text">
          Ensure tenants stay within their allocated messaging and bandwidth usage.
        </p>
        <div className="field">
          <label htmlFor="message_limit">Message Limit (per day)</label>
          <input
            id="message_limit"
            type="number"
            placeholder="50000"
            min={1}
            step={1}
            {...register("quotas.message_limit", { valueAsNumber: true })}
          />
          <p className="helper-text">
            Maximum MQTT messages the tenant can publish per day.
          </p>
          {errors.quotas?.message_limit && (
            <span className="error-text">{errors.quotas.message_limit.message}</span>
          )}
        </div>

        <div className="field">
          <label htmlFor="bandwidth_limit_gb">Bandwidth Limit (GB/month)</label>
          <input
            id="bandwidth_limit_gb"
            type="number"
            placeholder="100"
            min={0.1}
            step={0.1}
            {...register("quotas.bandwidth_limit_gb", { valueAsNumber: true })}
          />
          <p className="helper-text">
            Total data transfer allowance per month for the tenant.
          </p>
          {errors.quotas?.bandwidth_limit_gb && (
            <span className="error-text">
              {errors.quotas.bandwidth_limit_gb.message}
            </span>
          )}
        </div>
      </fieldset>

      <fieldset className="form-section">
        <legend>Features</legend>
        <p className="helper-text">
          Fine-tune availability of advanced capabilities for this tenant.
        </p>

        <Controller
          control={control}
          name="features.data_residency"
          render={({ field }) => (
            <div className="field">
              <span>Data Residency Regions</span>
              <div className="checkbox-group">
                {DATA_RESIDENCY_OPTIONS.map((region) => {
                  const checked = field.value?.includes(region) ?? false;
                  return (
                    <label key={region}>
                      <input
                        type="checkbox"
                        value={region}
                        checked={checked}
                        onChange={(event) => {
                          if (event.target.checked) {
                            field.onChange([...(field.value ?? []), region]);
                          } else {
                            field.onChange(
                              (field.value ?? []).filter((value) => value !== region),
                            );
                          }
                        }}
                      />
                      {region}
                    </label>
                  );
                })}
              </div>
              <p className="helper-text">
                Regions where the tenant&apos;s data is allowed to reside.
              </p>
              {errors.features?.data_residency && (
                <span className="error-text">
                  {errors.features.data_residency.message as string}
                </span>
              )}
            </div>
          )}
        />

        <div className="field">
          <label htmlFor="pii_redaction">
            <input
              id="pii_redaction"
              type="checkbox"
              {...register("features.pii_redaction")}
            />
            Enable PII Redaction
          </label>
          <p className="helper-text">
            Automatically scrub personally identifiable information from logs.
          </p>
          {errors.features?.pii_redaction && (
            <span className="error-text">
              {errors.features.pii_redaction.message as string}
            </span>
          )}
        </div>
      </fieldset>
    </div>
  );
}

export default TenantConfigForm;

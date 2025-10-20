# Geographic validation and data residency checks
# Provides reusable functions for enforcing GDPR-compliant data residency policies

package lib.geo

import rego.v1

# EU member states (27 countries as of 2025)
eu_countries := {
	"AT", "BE", "BG", "HR", "CY", "CZ", "DK", "EE", "FI", "FR",
	"DE", "GR", "HU", "IE", "IT", "LV", "LT", "LU", "MT", "NL",
	"PL", "PT", "RO", "SK", "SI", "ES", "SE",
}

# Check if a country code is in the EU
# Parameters:
#   country_code: Two-letter ISO country code (e.g., "DE", "FR")
# Returns: true if country is an EU member state
# Usage: geo.is_eu_country(input.environment.country)
is_eu_country(country_code) {
	country_code in eu_countries
}

# Check if a geo object represents an EU location
# Parameters:
#   geo: Object with country field (e.g., {"country": "DE", "city": "Berlin"})
# Returns: true if geo.country is in EU
# Usage: geo.is_eu_location(input.subject.location)
is_eu_location(geo) {
	geo.country in eu_countries
}

# Validate that resource region matches subject location region
# Parameters:
#   resource_region: Region string (e.g., "EU", "US", "APAC")
#   subject_location: Location region string
# Returns: true if both regions match
# Usage: geo.validate_data_residency(input.resource.region, input.subject.region)
validate_data_residency(resource_region, subject_location) {
	resource_region == subject_location
}

# Check if a country code is valid ISO 3166-1 alpha-2 format
# Parameters:
#   code: Country code string to validate
# Returns: true if code is exactly 2 uppercase letters (A-Z)
# Usage: geo.is_iso_alpha2("DE")
# Note: This only validates format, not whether the code is assigned
is_iso_alpha2(code) {
	count(code) == 2
	upper(code) == code
	regex.match(`^[A-Z]{2}$`, code)
}

# Normalize country code to uppercase ISO 3166-1 alpha-2 format
# Parameters:
#   code: Country code string (any case)
# Returns: Uppercase version of the code
# Usage: normalized := geo.normalize_country_code("de")
# Note: Use this before passing codes to is_eu_country for case-insensitive checks
normalize_country_code(code) := result {
	result := upper(code)
}

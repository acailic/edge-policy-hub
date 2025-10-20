# Unit tests for lib/geo.rego
# Tests geographic validation and data residency helpers

package lib.geo

import rego.v1

# Test is_eu_country with valid EU countries
test_is_eu_country_valid {
	is_eu_country("DE")
	is_eu_country("FR")
	is_eu_country("NL")
}

# Test is_eu_country with non-EU countries
test_is_eu_country_invalid {
	not is_eu_country("US")
	not is_eu_country("GB") # UK is not EU post-Brexit
	not is_eu_country("CH") # Switzerland is not EU
}

# Test is_eu_country is case-sensitive (uppercase expected)
test_is_eu_country_case_sensitive {
	is_eu_country("DE")
	not is_eu_country("de") # lowercase should fail
}

# Test is_eu_location with country field
test_is_eu_location_with_country_field {
	geo := {"country": "DE", "city": "Berlin"}
	is_eu_location(geo)
}

# Test is_eu_location with non-EU country
test_is_eu_location_non_eu {
	geo := {"country": "US"}
	not is_eu_location(geo)
}

# Test validate_data_residency with matching regions
test_validate_data_residency_match {
	validate_data_residency("EU", "EU")
	validate_data_residency("US", "US")
}

# Test validate_data_residency with mismatched regions
test_validate_data_residency_mismatch {
	not validate_data_residency("EU", "US")
	not validate_data_residency("US", "EU")
}

# Test eu_countries set completeness
test_eu_countries_set_completeness {
	count(eu_countries) == 27
	"DE" in eu_countries
	"FR" in eu_countries
}

# Test is_iso_alpha2 with valid codes
test_is_iso_alpha2_valid {
	is_iso_alpha2("DE")
	is_iso_alpha2("US")
	is_iso_alpha2("FR")
	is_iso_alpha2("GB")
}

# Test is_iso_alpha2 with invalid codes (lowercase)
test_is_iso_alpha2_lowercase {
	not is_iso_alpha2("de")
	not is_iso_alpha2("us")
}

# Test is_iso_alpha2 with invalid codes (wrong length)
test_is_iso_alpha2_wrong_length {
	not is_iso_alpha2("D")
	not is_iso_alpha2("DEU")
	not is_iso_alpha2("")
}

# Test is_iso_alpha2 with invalid codes (non-alpha)
test_is_iso_alpha2_non_alpha {
	not is_iso_alpha2("D1")
	not is_iso_alpha2("12")
	not is_iso_alpha2("D-")
}

# Test normalize_country_code
test_normalize_country_code {
	normalize_country_code("de") == "DE"
	normalize_country_code("us") == "US"
	normalize_country_code("DE") == "DE"
	normalize_country_code("fr") == "FR"
}

# Test is_eu_country with normalized codes
test_is_eu_country_with_normalization {
	normalized := normalize_country_code("de")
	is_eu_country(normalized)

	normalized_fr := normalize_country_code("fr")
	is_eu_country(normalized_fr)
}

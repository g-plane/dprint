use dprint_core::configuration::{ConfigKeyMap, GlobalConfiguration, ResolveGlobalConfigOptions};
use dprint_core::types::ErrBox;
use std::collections::HashMap;

use super::{ConfigMap, ConfigMapValue};
use crate::environment::Environment;

pub struct GetGlobalConfigOptions {
  pub check_unknown_property_diagnostics: bool,
}

pub fn get_global_config(config_map: ConfigMap, environment: &impl Environment, options: &GetGlobalConfigOptions) -> Result<GlobalConfiguration, ErrBox> {
  match get_global_config_inner(config_map, environment, options) {
    Ok(config) => Ok(config),
    Err(err) => err!("Error resolving global config from configuration file. {}", err.to_string()),
  }
}

fn get_global_config_inner(config_map: ConfigMap, environment: &impl Environment, options: &GetGlobalConfigOptions) -> Result<GlobalConfiguration, ErrBox> {
  // now get and resolve the global config
  let global_config = get_global_config_from_config_map(config_map, options)?;
  let global_config_result = dprint_core::configuration::resolve_global_config(
    global_config,
    &ResolveGlobalConfigOptions {
      check_unknown_property_diagnostics: options.check_unknown_property_diagnostics,
    },
  );

  // check global diagnostics
  let mut diagnostic_count = 0;
  if !global_config_result.diagnostics.is_empty() {
    for diagnostic in &global_config_result.diagnostics {
      environment.log_error(&diagnostic.message);
      diagnostic_count += 1;
    }
  }

  return if diagnostic_count > 0 {
    err!("Had {} config diagnostic(s).", diagnostic_count)
  } else {
    Ok(global_config_result.config)
  };

  fn get_global_config_from_config_map(config_map: ConfigMap, options: &GetGlobalConfigOptions) -> Result<ConfigKeyMap, ErrBox> {
    // at this point, there should only be key values inside the hash map
    let mut global_config = HashMap::new();

    for (key, value) in config_map.into_iter() {
      if key == "$schema" {
        continue;
      } // ignore $schema property

      if let ConfigMapValue::KeyValue(value) = value {
        global_config.insert(key, value);
      } else if options.check_unknown_property_diagnostics {
        return err!("Unexpected non-string, boolean, or int property '{}'.", key);
      }
    }

    Ok(global_config)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::configuration::ConfigMap;
  use crate::environment::TestEnvironment;
  use std::collections::HashMap;

  #[test]
  fn it_should_get_global_config() {
    let mut config_map = HashMap::new();
    config_map.insert(String::from("lineWidth"), ConfigMapValue::from_i32(80));
    assert_gets(
      config_map,
      GlobalConfiguration {
        line_width: Some(80),
        use_tabs: None,
        indent_width: None,
        new_line_kind: None,
      },
    );
  }

  #[test]
  fn it_should_error_on_unexpected_object_properties_when_check_unknown_property_diagnostics_true() {
    let mut config_map = HashMap::new();
    config_map.insert(String::from("test"), ConfigMapValue::HashMap(HashMap::new()));
    assert_errors_with_options(
      config_map,
      vec![],
      "Unexpected non-string, boolean, or int property 'test'.",
      &GetGlobalConfigOptions {
        check_unknown_property_diagnostics: true,
      },
    );
  }

  #[test]
  fn it_should_not_error_on_unexpected_object_properties_when_check_unknown_property_diagnostics_false() {
    let mut config_map = HashMap::new();
    config_map.insert(String::from("test"), ConfigMapValue::HashMap(HashMap::new()));
    assert_gets_with_options(
      config_map,
      GlobalConfiguration {
        line_width: None,
        use_tabs: None,
        indent_width: None,
        new_line_kind: None,
      },
      &GetGlobalConfigOptions {
        check_unknown_property_diagnostics: false,
      },
    );
  }

  #[test]
  fn it_should_log_config_file_diagnostics() {
    let mut config_map = HashMap::new();
    config_map.insert(String::from("lineWidth"), ConfigMapValue::from_str("test"));
    config_map.insert(String::from("unknownProperty"), ConfigMapValue::from_i32(80));
    assert_errors(
      config_map,
      vec![
        "Error parsing configuration value for 'lineWidth'. Message: invalid digit found in string",
        "Unknown property in configuration: unknownProperty",
      ],
      "Had 2 config diagnostic(s).",
    );
  }

  #[test]
  fn it_should_ignore_schema_property() {
    let mut config_map = HashMap::new();
    config_map.insert(String::from("$schema"), ConfigMapValue::from_str("test"));
    assert_gets(
      config_map,
      GlobalConfiguration {
        line_width: None,
        use_tabs: None,
        indent_width: None,
        new_line_kind: None,
      },
    );
  }

  fn assert_gets(config_map: ConfigMap, global_config: GlobalConfiguration) {
    assert_gets_with_options(
      config_map,
      global_config,
      &GetGlobalConfigOptions {
        check_unknown_property_diagnostics: true,
      },
    )
  }

  fn assert_gets_with_options(config_map: ConfigMap, global_config: GlobalConfiguration, options: &GetGlobalConfigOptions) {
    let test_environment = TestEnvironment::new();
    let result = get_global_config(config_map, &test_environment, &options).unwrap();
    assert_eq!(result, global_config);
  }

  fn assert_errors(config_map: ConfigMap, logged_errors: Vec<&'static str>, message: &str) {
    assert_errors_with_options(
      config_map,
      logged_errors,
      message,
      &GetGlobalConfigOptions {
        check_unknown_property_diagnostics: true,
      },
    );
  }

  fn assert_errors_with_options(config_map: ConfigMap, logged_errors: Vec<&'static str>, message: &str, options: &GetGlobalConfigOptions) {
    let test_environment = TestEnvironment::new();
    let result = get_global_config(config_map, &test_environment, options);
    assert_eq!(
      result.err().unwrap().to_string(),
      format!("Error resolving global config from configuration file. {}", message)
    );
    assert_eq!(test_environment.take_stderr_messages(), logged_errors);
  }
}

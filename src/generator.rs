use crate::config_parser::ProjectConfig;
use crate::model::{CStandard, CppStandard, Language};
use crate::util::get_cmake_version;
use git2::Repository;
use indoc::{formatdoc, indoc};
use std::io::Error;
use std::process::{exit, ExitStatus};
use std::{fmt::Display, fs, fs::File, io::Write, process::Command};

const EXAMPLE_C_PROGRAM: &'static str = indoc! {r#"
#include <stdio.h>
#include "example.h"

int main() {
    example_function();
    return 0;
}
"#};

const EXAMPLE_CPP_PROGRAM: &'static str = indoc! {r#"
#include <iostream>
#include "example.hpp"

using std::cout;

int main() {
    example_function();
    return 0;
}
"#};

const EXAMPLE_C_HEADER: &'static str = indoc! {r#"
#ifndef EXAMPLE_H
#define EXAMPLE_H

#include <stdio.h>

void example_function();

#endif
"#};

const EXAMPLE_CPP_HEADER: &'static str = indoc! {r#"
#ifndef EXAMPLE_HPP
#define EXAMPLE_HPP

#include <iostream>

void example_function();

#endif
"#};

const EXAMPLE_C_IMPLEMENTATION: &'static str = indoc! {r#"
#include "example.h"

void example_function() {
    printf("Hello from example_function!\n");
}
"#};

const EXAMPLE_CPP_IMPLEMENTATION: &'static str = indoc! {r#"
#include "example.hpp"

void example_function() {
    std::cout << "Hello from example_function!\n";
}
"#};

pub fn generate_project(
    path: String,
    project_name: String,
    lang: Language,
) -> Result<(), Error> {
    let src_path = format!("{}/src", path);
    let include_path = format!("{}/include", path);
    fs::create_dir_all(&src_path)?;
    fs::create_dir_all(&include_path)?;

    let cmakelists_path = format!("{path}/CMakeLists.txt");
    let mut cmakelists_file = File::create(cmakelists_path)?;

    match lang {
        Language::C(standard) => {
            let main_path = format!("{}/main.c", src_path);
            let header_path = format!("{}/example.h", include_path);
            let implementation_path = format!("{}/example.c", src_path);

            let mut main_file = File::create(main_path)?;
            let mut header_file = File::create(header_path)?;
            let mut implementation_file = File::create(implementation_path)?;

            cmakelists_file.write_all(get_c_cmakelists(&project_name, standard).as_bytes())?;

            main_file.write_all(EXAMPLE_C_PROGRAM.as_bytes())?;
            header_file.write_all(EXAMPLE_C_HEADER.as_bytes())?;
            implementation_file.write_all(EXAMPLE_C_IMPLEMENTATION.as_bytes())?;
        }
        Language::CPP(standard) => {
            let main_path = format!("{}/main.cpp", src_path);
            let header_path = format!("{}/example.hpp", include_path);
            let implementation_path = format!("{}/example.cpp", src_path);

            let mut main_file = File::create(main_path)?;
            let mut header_file = File::create(header_path)?;
            let mut implementation_file = File::create(implementation_path)?;

            cmakelists_file.write_all(get_cpp_cmakelists(&project_name, standard).as_bytes())?;

            main_file.write_all(EXAMPLE_CPP_PROGRAM.as_bytes())?;
            header_file.write_all(EXAMPLE_CPP_HEADER.as_bytes())?;
            implementation_file.write_all(EXAMPLE_CPP_IMPLEMENTATION.as_bytes())?;
        }
    }

    let config = ProjectConfig::create_new_config(&project_name, lang, "src", "include", "build");
    let config_path = format!("{}/.c3pm.toml", path);
    let mut config_file = File::create(config_path)?;
    config_file.write_all(ProjectConfig::serialize_config(&config)?.as_bytes())?;

    if let Err(e) = Repository::init(&path) {
        eprintln!("Failed to initialize git repository: {}", e);
    }

    let cmake_status = configure_cmake_project(&path);

    if let Err(e) = cmake_status {
        eprintln!("Failed to configure CMake project: {}", e);
        return Err(Error::new(
            std::io::ErrorKind::Other,
            format!("CMake configure command failed: {}", e),
        ));
    }

    Ok(())
}

pub fn configure_cmake_project(path: &String) -> Result<ExitStatus, Error> {
    let cmake_status = Command::new("cmake")
        .arg("-S")
        .arg(&path)
        .arg("-B")
        .arg(format!("{}/build", path))
        .status();
    cmake_status
}

fn get_c_cmakelists(project_name: &String, standard: CStandard) -> String {
    get_common_cmakelists(
        project_name,
        format!("set(CMAKE_C_STANDARD {standard})"),
        "SOURCES",
        "HEADERS",
    )
}

fn get_cpp_cmakelists(project_name: &String, standard: CppStandard) -> String {
    get_common_cmakelists(
        project_name,
        format!("set(CMAKE_CXX_STANDARD {standard})"),
        "SOURCES",
        "HEADERS",
    )
}

fn get_common_cmakelists<Env: Display>(
    project_name: &str,
    env: Env,
    sources_var: &str,
    headers_var: &str,
) -> String {
    let cmake_version = match get_cmake_version() {
        Ok(version) => format!("{}", version),
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(127);
        }
    };

    formatdoc! {r#"
        cmake_minimum_required(VERSION {cmake_version})

        project({project_name})

        {env}

        set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

        file(GLOB_RECURSE {sources_var} CONFIGURE_DEPENDS "src/*.c" "src/*.cpp")
        file(GLOB_RECURSE {headers_var} CONFIGURE_DEPENDS "include/*.h" "include/*.hpp")

        add_executable({project_name} ${{{sources_var}}})
        target_include_directories({project_name} PRIVATE include)
    "#}
}

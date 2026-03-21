Name:           tascli
Version:        %{version}
Release:        1%{?dist}
Summary:        A simple CLI tool for tracking tasks and records from terminal
License:        MIT
URL:            https://github.com/Aperocky/tascli

%description
A small, simple, fast, local CLI tool for tracking tasks and records from unix terminal.

%install
install -Dm755 %{source_bindir}/%{name} %{buildroot}/usr/local/bin/%{name}

%files
/usr/local/bin/%{name}

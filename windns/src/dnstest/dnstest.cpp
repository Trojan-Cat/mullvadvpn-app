// dnstest.cpp : Defines the entry point for the console application.
//

#include "stdafx.h"
#include "windns/wmi/connection.h"
#include "windns/wmi/resultset.h"
#include "windns/wmi/methodcall.h"
#include "windns/comhelpers.h"

#include <iostream>
#include <algorithm>
#include <vector>
#include <string>
#include <atlsafe.h>
#include "windns/dnsconfiguration.h"



int main()
{
	auto conn = wmi::Connection(wmi::Connection::Namespace::Cimv2);

	auto resultSet = conn.query(L"SELECT * from Win32_NetworkAdapterConfiguration WHERE IPEnabled = True");

	std::vector<DnsConfiguration> configs;

	while (resultSet.advance())
	{
		auto object = resultSet.result();

		DnsConfiguration config(object);

		if (config.interfaceIndex() != 18)
		{
			continue;
		}

		wmi::MethodCall methodCall;

		{
			std::vector<std::wstring> servers;

			servers.push_back(L"8.8.8.8");
			servers.push_back(L"1.1.1.1");

			auto comServers = ComConvertIntoStringArray(servers);

			methodCall.addArgument(L"DNSServerSearchOrder", ComPackageStringArray(comServers));
		}

		auto status = methodCall.invoke(conn, object, L"SetDNSServerSearchOrder");



		configs.emplace_back(DnsConfiguration(resultSet.result()));
	}

	return 0;
}


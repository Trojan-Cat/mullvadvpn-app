//
//  ChainedError+Logger.swift
//  MullvadVPN
//
//  Created by pronebird on 02/08/2020.
//  Copyright © 2020 Mullvad VPN AB. All rights reserved.
//

import Foundation
import Logging

extension Logger {

    func error(chainedError: ChainedError,
               message: @autoclosure () -> String? = nil,
               metadata: @autoclosure () -> Logger.Metadata? = nil,
               source: @autoclosure () -> String? = nil,
               file: String = #file, function: String = #function, line: UInt = #line)
    {
        let s = Message(stringLiteral: chainedError.displayChain(message: message()))
        log(level: .error, s, metadata: metadata(), source: source(), file: file, function: function, line: line)
    }

}

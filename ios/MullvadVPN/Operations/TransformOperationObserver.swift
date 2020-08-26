//
//  TransformOperationObserver.swift
//  MullvadVPN
//
//  Created by pronebird on 06/07/2020.
//  Copyright © 2020 Mullvad VPN AB. All rights reserved.
//

import Foundation

/// A private type erasing observer that type casts the input operation type to the expected
/// operation type before calling the wrapped observer
class TransformOperationObserver<S: OperationProtocol>: OperationObserver {
    private let willFinish: (S, Error?) -> Void
    private let didFinish: (S, Error?) -> Void

    init<T: OperationObserver>(_ observer: T) {
        willFinish = Self.wrap(observer.operationWillFinish)
        didFinish = Self.wrap(observer.operationDidFinish)
    }

    func operationWillFinish(_ operation: S, error: Error?) {
        willFinish(operation, error)
    }

    func operationDidFinish(_ operation: S, error: Error?) {
        didFinish(operation, error)
    }

    private class func wrap<U>(_ body: @escaping (U, Error?) -> Void) -> (S, Error?) -> Void {
        return { (operation: S, error: Error?) in
            if let transformed = operation as? U {
                body(transformed, error)
            } else {
                fatalError("\(Self.self) failed to cast \(S.self) to \(U.self)")
            }
        }
    }
}

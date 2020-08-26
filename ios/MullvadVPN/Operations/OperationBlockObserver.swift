//
//  OperationBlockObserver.swift
//  MullvadVPN
//
//  Created by pronebird on 06/07/2020.
//  Copyright © 2020 Mullvad VPN AB. All rights reserved.
//

import Foundation

class OperationBlockObserver<OperationType: OperationProtocol>: OperationObserver {
    private var willFinish: ((OperationType, Error?) -> Void)?
    private var didFinish: ((OperationType, Error?) -> Void)?

    let queue: DispatchQueue?

    init(queue: DispatchQueue? = nil, willFinish: ((OperationType, Error?) -> Void)? = nil, didFinish: ((OperationType, Error?) -> Void)? = nil) {
        self.queue = queue
        self.willFinish = willFinish
        self.didFinish = didFinish
    }

    func operationWillFinish(_ operation: OperationType, error: Error?) {
        if let willFinish = self.willFinish {
            scheduleEvent {
                willFinish(operation, error)
            }
        }
    }

    func operationDidFinish(_ operation: OperationType, error: Error?) {
        if let didFinish = self.didFinish {
            scheduleEvent {
                didFinish(operation, error)
            }
        }
    }

    private func scheduleEvent(_ body: @escaping () -> Void) {
        if let queue = queue {
            queue.async(execute: body)
        } else {
            body()
        }
    }
}

extension OperationProtocol {
    func addDidFinishBlockObserver(queue: DispatchQueue? = nil, _ block: @escaping (Self, Error?) -> Void) {
        addObserver(OperationBlockObserver(queue: queue, didFinish: block))
    }
}

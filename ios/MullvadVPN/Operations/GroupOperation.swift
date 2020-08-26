//
//  GroupOperation.swift
//  MullvadVPN
//
//  Created by pronebird on 20/08/2020.
//  Copyright © 2020 Mullvad VPN AB. All rights reserved.
//

import Foundation

class GroupOperation: AsyncOperation {
    private let operationQueue = OperationQueue()
    private let childLock = NSRecursiveLock()
    private var children: Set<Operation> = []
    private var childError: Error?

    init(underlyingQueue: DispatchQueue? = nil, operations: [Operation]) {
        operationQueue.underlyingQueue = underlyingQueue
        operationQueue.isSuspended = true

        super.init()

        addChildren(operations)
    }

    deinit {
        operationQueue.cancelAllOperations()
        operationQueue.isSuspended = false
    }

    override func main() {
        operationQueue.isSuspended = false
    }

    override func operationDidCancel(error: Error?) {
        children.forEach { $0.cancel() }
    }

    func addChildren(_ operations: [Operation]) {
        childLock.lock()
        defer { childLock.unlock() }

        precondition(!self.isFinished, "Children cannot be added after the GroupOperation has finished.")

        children.formUnion(operations)

        let completionOperation = BlockOperation { [weak self] in
            self?._childrenDidFinish(operations)
        }

        operations.forEach { completionOperation.addDependency($0) }

        self.operationQueue.addOperations(operations, waitUntilFinished: false)
        self.operationQueue.addOperation(completionOperation)
    }

    // MARK: - Private

    private func _childrenDidFinish(_ children: [Operation]) {
        childLock.lock()
        self.children.subtract(children)

        // Collect the first child error
        if childError == nil {
            let childErrors = children.compactMap { (op) -> Error? in
                return (op as? OperationProtocol)?.error
            }
            childError = childErrors.first
        }

        if children.isEmpty {
            finish(error: childError)
        }

        childLock.unlock()
    }
}

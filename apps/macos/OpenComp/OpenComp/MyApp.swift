import SwiftUI

@main
struct MyApp: App {
    var body: some Scene {
        MenuBarExtra("OpenComp", image: "MenuBarIcon") {
            BuddyView()
        }
        .menuBarExtraStyle(.window)
    }
}

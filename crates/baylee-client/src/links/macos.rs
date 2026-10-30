//! macOS hands an opened link to the app as an Apple Event (`'GURL'`), on a
//! cold start as much as while it runs, and always to the client (the
//! process that checked in with the window server), not to the launcher that
//! started it: measured on 30.10.2026 with the launcher's layout, the client
//! started from the original bundle and from a downloaded generation's copy.
//!
//! winit 0.30 owns the app's delegate (its `ApplicationDelegate` panics when
//! it finds another), so the link is asked for from `NSAppleEventManager`
//! instead, which needs no delegate. Asked before the event loop runs, and
//! again when `AppKit` says it will finish launching, which is the documented
//! moment: a later `AppKit` that installs a default handler of its own in
//! between cannot take the cold start's link away.

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::NSApplicationWillFinishLaunchingNotification;
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter,
};

/// `'GURL'`: both the event's class (`kInternetEventClass`) and its id
/// (`kAEGetURL`).
const GURL: u32 = u32::from_be_bytes(*b"GURL");
/// `'----'`: the event's direct object, the link itself (`keyDirectObject`).
const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `LinkEvents`
    // implements no `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "BayleeLinkEvents"]
    struct LinkEvents;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for LinkEvents {}

    impl LinkEvents {
        // SAFETY: the signature NSAppleEventManager calls a handler with:
        // the event and its reply, both descriptors, and no result.
        #[unsafe(method(handleGetURL:withReplyEvent:))]
        fn handle_get_url(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            // SAFETY: `paramDescriptorForKeyword:` takes an `AEKeyword` (a
            // four-character code, `u32`) and answers a descriptor or nil.
            let link: Option<Retained<NSAppleEventDescriptor>> =
                unsafe { msg_send![event, paramDescriptorForKeyword: DIRECT_OBJECT] };
            if let Some(text) = link.and_then(|link| link.stringValue()) {
                super::arrived(text.to_string());
            }
        }

        // SAFETY: a notification observer's selector takes the notification
        // and answers nothing.
        #[unsafe(method(willFinishLaunching:))]
        fn will_finish_launching(&self, _note: &NSNotification) {
            self.ask();
        }
    }
);

impl LinkEvents {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: NSObject's `init` has this signature.
        unsafe { msg_send![super(this), init] }
    }

    /// Asks for `'GURL'` events; asking again replaces the earlier handler.
    fn ask(&self) {
        let manager = NSAppleEventManager::sharedAppleEventManager();
        // SAFETY: the handler is `self`, which lives for the process (see
        // `listen`); the selector is one it implements with the documented
        // signature; the class and id are four-character codes (`u32`).
        unsafe {
            let () = msg_send![
                &*manager,
                setEventHandler: self,
                andSelector: sel!(handleGetURL:withReplyEvent:),
                forEventClass: GURL,
                andEventID: GURL
            ];
        }
    }
}

/// Asks for links before the event loop runs. Bevy calls a plugin's
/// `finish` on the main thread, after winit made the app.
pub(super) fn listen() {
    let Some(mtm) = MainThreadMarker::new() else {
        bevy::log::warn!("links: not on the main thread; macOS links are not heard");
        return;
    };
    let events = LinkEvents::new(mtm);
    events.ask();
    let center = NSNotificationCenter::defaultCenter();
    // SAFETY: the observer implements `willFinishLaunching:` taking the
    // notification; it is never removed and never freed (below), so the
    // center never calls a freed object. The name is AppKit's own constant,
    // read once.
    unsafe {
        center.addObserver_selector_name_object(
            &events,
            sel!(willFinishLaunching:),
            Some(NSApplicationWillFinishLaunchingNotification),
            None,
        );
    }
    // The event manager and the notification center hold their handler
    // without retaining it: this one lives as long as the process.
    std::mem::forget(events);
}

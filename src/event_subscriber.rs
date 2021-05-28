pub struct EventSubscriber<'a> {
	on_download: &'a (dyn Fn(&str) + Sync),
	on_convert: &'a (dyn Fn(&str) + Sync),
	on_finish: &'a (dyn Fn(&str) + Sync),
	on_failed: &'a (dyn Fn(&str) + Sync),
}

//TODO: Find better solution.
impl<'a> EventSubscriber<'a> {
	// Create new EventSubscriber with the specified callbacks.
	pub fn new(
		on_download: &'a (dyn Fn(&str) + Sync),
		on_convert: &'a (dyn Fn(&str) + Sync),
		on_finish: &'a (dyn Fn(&str) + Sync),
		on_failed: &'a (dyn Fn(&str) + Sync),
	) -> Self {
		EventSubscriber {
			on_download,
			on_convert,
			on_finish,
			on_failed,
		}
	}

	// Fires when a download is starting.
	pub fn on_download(&self, media_name: &str) {
		let f = self.on_download;
		f(media_name);
	}

	/// Fires when a conversion starts.
	pub fn on_convert(&self, media_name: &str) {
		let f = self.on_convert;
		f(media_name);
	}

	/// Fires when a download is finished.
	pub fn on_finish(&self, media_name: &str) {
		let f = self.on_finish;
		f(media_name);
	}

	/// Fires when a download has failed.
	pub fn on_failed(&self, media_name: &str) {
		let f = self.on_failed;
		f(media_name);
	}
}

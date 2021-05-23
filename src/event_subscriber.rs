pub struct EventSubscriber<'a> {
	on_download: &'a (dyn Fn(&str) + Sync),
	on_finish: &'a (dyn Fn(&str) + Sync),
	on_failed: &'a (dyn Fn(&str) + Sync),
}

impl<'a> EventSubscriber<'a> {
	pub fn new(
		on_download: &'a (dyn Fn(&str) + Sync),
		on_finish: &'a (dyn Fn(&str) + Sync),
		on_failed: &'a (dyn Fn(&str) + Sync),
	) -> Self {
		EventSubscriber {
			on_download,
			on_finish,
			on_failed,
		}
	}

	pub fn on_download(&self, media_name: &str) {
		let f = self.on_download;
		f(media_name);
	}

	pub fn on_finish(&self, media_name: &str) {
		let f = self.on_finish;
		f(media_name);
	}

	pub fn on_failed(&self, media_name: &str) {
		let f = self.on_failed;
		f(media_name);
	}
}

use core_done::service::Service;
use relm4::{
	component::{AsyncComponent, AsyncComponentParts},
	factory::{AsyncFactoryVecDeque, DynamicIndex},
	gtk::{self, prelude::OrientableExt, prelude::WidgetExt},
	AsyncComponentSender, RelmWidgetExt,
};

use crate::{
	app::factories::service::{ServiceFactoryModel, ServiceFactoryOutput},
	fl,
};

pub struct ServicesSidebarModel {
	services_factory: AsyncFactoryVecDeque<ServiceFactoryModel>,
}

#[derive(Debug)]
pub enum ServicesSidebarInput {
	ServiceSelected(DynamicIndex, Service),
	ReloadSidebar(Service),
}

#[derive(Debug)]
pub enum ServicesSidebarOutput {
	ServiceSelected(Service),
	ServiceDisabled(Service),
}

#[relm4::component(pub async)]
impl AsyncComponent for ServicesSidebarModel {
	type CommandOutput = ();
	type Input = ServicesSidebarInput;
	type Output = ServicesSidebarOutput;
	type Init = ();

	view! {
		#[root]
		gtk::ScrolledWindow {
			set_height_request: 75,
			set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Never),
			set_margin_all: 10,
			#[local_ref]
			services_list -> gtk::FlowBox {
				set_valign: gtk::Align::Start,
				set_orientation: gtk::Orientation::Vertical,
				set_selection_mode: gtk::SelectionMode::Single,
				set_homogeneous: true,
				set_max_children_per_line: 2,
			},
		}
	}

	async fn init(
		_init: Self::Init,
		root: Self::Root,
		sender: AsyncComponentSender<Self>,
	) -> AsyncComponentParts<Self> {
		let _keyboard_shortcuts: &str = fl!("keyboard-shortcuts");
		let _about_done: &str = fl!("about-done");
		let _quit: &str = fl!("quit");

		let mut services_factory = AsyncFactoryVecDeque::builder()
			.launch(gtk::FlowBox::default())
			.forward(sender.input_sender(), |output| match output {
				ServiceFactoryOutput::ServiceSelected(index, service) => {
					ServicesSidebarInput::ServiceSelected(index, service)
				},
			});

		{
			let mut guard = services_factory.guard();

			for service in Service::list() {
				if service.get_service().available() {
					guard.push_back(service);
				}
			}
		}

		let model = ServicesSidebarModel { services_factory };

		let services_list = model.services_factory.widget();

		let selected_child = services_list.child_at_index(0).unwrap();
		services_list.select_child(&selected_child);

		let widgets = view_output!();

		AsyncComponentParts { model, widgets }
	}

	async fn update(
		&mut self,
		message: Self::Input,
		sender: AsyncComponentSender<Self>,
		_root: &Self::Root,
	) {
		match message {
			ServicesSidebarInput::ReloadSidebar(service) => {
				let mut guard = self.services_factory.guard();
				guard.clear();
				for service in Service::list() {
					if service.get_service().available() {
						guard.push_back(service);
					}
				}
				sender
					.output(ServicesSidebarOutput::ServiceDisabled(service))
					.unwrap()
			},
			ServicesSidebarInput::ServiceSelected(index, service) => {
				let flow_box = self.services_factory.widget();
				let selected_child = flow_box
					.child_at_index(index.current_index() as i32)
					.unwrap();
				flow_box.select_child(&selected_child);
				sender
					.output(ServicesSidebarOutput::ServiceSelected(service))
					.unwrap();
			},
		}
	}
}

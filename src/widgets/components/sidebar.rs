use super::smart_lists::{SmartList, SmartListModel, SmartListOutput};
use crate::application::plugin::Plugin;
use crate::fl;
use crate::widgets::factory::list::ListData;
use crate::widgets::factory::provider::{
	PluginInit, ProviderInput, ProviderModel,
};
use proto_rust::provider::List;
use relm4::adw::traits::PreferencesGroupExt;
use relm4::component::{
	AsyncComponentParts, AsyncComponentSender, SimpleAsyncComponent,
};
use relm4::factory::AsyncFactoryVecDeque;
use relm4::{adw, Component, ComponentController, Controller};
use relm4::{
	gtk,
	gtk::prelude::{BoxExt, OrientableExt, WidgetExt},
	RelmWidgetExt,
};

#[derive(Debug)]
pub struct SidebarModel {
	provider_factory: AsyncFactoryVecDeque<ProviderModel>,
	smart_list_controller: Controller<SmartListModel>,
}

#[derive(Debug)]
pub enum SidebarInput {
	AddListToProvider(usize, String, String),
	ListSelected(ListData),
	EnableService(Plugin),
	DisableService(Plugin),
	Forward,
	Notify(String),
	SelectSmartList(SmartList),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SidebarOutput {
	ListSelected(ListData),
	Forward,
	Notify(String),
	DisablePlugin,
	SelectSmartList(SmartList),
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for SidebarModel {
	type Input = SidebarInput;
	type Output = SidebarOutput;
	type Widgets = SidebarWidgets;
	type Init = ();

	view! {
		sidebar = &gtk::Box {
			set_orientation: gtk::Orientation::Vertical,
			#[name(scroll_window)]
			gtk::ScrolledWindow {
				#[name(clamp)]
				adw::Clamp {
					#[wrap(Some)]
					set_child = &gtk::Box {
						set_orientation: gtk::Orientation::Vertical,
						set_css_classes: &["navigation-sidebar"],
						set_margin_top: 5,
						set_margin_start: 10,
						set_margin_end: 10,
						set_spacing: 12,
						set_vexpand: true,
						append = model.smart_list_controller.widget(),
						#[local_ref]
						providers_container -> adw::PreferencesGroup {
							set_hexpand: false,
							set_title: "Services"
						},
						gtk::CenterBox {
							#[watch]
							set_visible: false, // TODO: Show when no provider is enabled.
							set_orientation: gtk::Orientation::Vertical,
							set_halign: gtk::Align::Center,
							set_valign: gtk::Align::Center,
							set_vexpand: true,
							#[wrap(Some)]
							set_center_widget = &gtk::Box {
								set_orientation: gtk::Orientation::Vertical,
								set_spacing: 24,
								gtk::Picture {
									set_resource: Some("/dev/edfloreshz/Done/icons/scalable/actions/leaf.png"),
									set_margin_all: 25
								},
								gtk::Label {
									set_label: fl!("empty-sidebar"),
									set_css_classes: &["title-4", "accent"],
									set_wrap: true
								},
								gtk::Label {
									set_label: fl!("open-preferences"),
									set_wrap: true
								}
							}
						}
					}
				}
			},
		}
	}

	async fn init(
		_init: Self::Init,
		root: Self::Root,
		sender: AsyncComponentSender<Self>,
	) -> AsyncComponentParts<Self> {
		let mut model = SidebarModel {
			provider_factory: AsyncFactoryVecDeque::new(
				adw::PreferencesGroup::default(),
				sender.input_sender(),
			),
			smart_list_controller: SmartListModel::builder().launch(()).forward(
				sender.input_sender(),
				|message| match message {
					SmartListOutput::SelectSmartList(list) => {
						SidebarInput::SelectSmartList(list)
					},
					SmartListOutput::Forward => SidebarInput::Forward,
				},
			),
		};

		let providers_container = model.provider_factory.widget();

		let widgets = view_output!();

		for plugin in Plugin::get_plugins().unwrap() {
			if let Ok(service) = plugin.connect().await {
				model
					.provider_factory
					.guard()
					.push_back(PluginInit::new(plugin.clone(), service));
				info!("Added {:?} service to the sidebar", plugin.name)
			} else {
				error!("{} service is not reachable.", plugin.name)
			}
		}

		AsyncComponentParts { model, widgets }
	}

	async fn update(
		&mut self,
		message: Self::Input,
		sender: AsyncComponentSender<Self>,
	) {
		match message {
			SidebarInput::AddListToProvider(index, provider_id, name) => {
				match Plugin::get_plugins()
					.unwrap()
					.iter()
					.find(|i| i.id == provider_id)
				{
					Some(provider) => match provider.connect().await {
						Ok(mut service) => {
							let list = List::new(&name, &provider_id);
							match service.create_list(list.clone()).await {
								Ok(response) => {
									let response = response.into_inner();
									if response.successful {
										self
											.provider_factory
											.send(index, ProviderInput::AddList(list));
									}
									sender
										.output(SidebarOutput::Notify(response.message))
										.unwrap_or_default();
								},
								Err(err) => {
									sender
										.output(SidebarOutput::Notify(err.to_string()))
										.unwrap_or_default();
								},
							}
						},
						Err(err) => {
							sender
								.output(SidebarOutput::Notify(err.to_string()))
								.unwrap_or_default();
						},
					},
					None => {
						sender
							.output(SidebarOutput::Notify("Provider not found".to_string()))
							.unwrap_or_default();
					},
				}
			},
			SidebarInput::EnableService(plugin) => {
				if plugin.is_running() {
					let index = Plugin::get_plugins()
						.unwrap()
						.iter()
						.position(|p| p == &plugin)
						.unwrap();
					self.provider_factory.send(index, ProviderInput::Enable)
				}
			},
			SidebarInput::DisableService(plugin) => {
				let index = Plugin::get_plugins()
					.unwrap()
					.iter()
					.position(|p| p == &plugin)
					.unwrap();
				self.provider_factory.send(index, ProviderInput::Disable);
				sender
					.output(SidebarOutput::DisablePlugin)
					.unwrap_or_default()
			},
			SidebarInput::ListSelected(list) => {
				sender
					.output(SidebarOutput::ListSelected(list))
					.unwrap_or_default();
			},
			SidebarInput::Forward => {
				sender.output(SidebarOutput::Forward).unwrap_or_default();
			},
			SidebarInput::Notify(msg) => {
				sender
					.output(SidebarOutput::Notify(msg))
					.unwrap_or_default();
			},
			SidebarInput::SelectSmartList(list) => sender
				.output(SidebarOutput::SelectSmartList(list))
				.unwrap_or_default(),
		}
	}
}

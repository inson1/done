use std::str::FromStr;

use crate::fl;
use adw::glib::Propagation;
use adw::prelude::{
	ActionRowExt, BoxExt, ExpanderRowExt, OrientableExt, ToggleButtonExt,
};
use adw::prelude::{EntryRowExt, PreferencesRowExt};
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use core_done::models::list::List;
use core_done::models::priority::Priority;
use core_done::models::recurrence::Day;
use core_done::models::status::Status;
use core_done::models::task::Task;
use relm4::factory::{AsyncFactoryComponent, FactoryVecDeque};
use relm4::factory::{AsyncFactorySender, DynamicIndex, FactoryView};
use relm4::{
	adw, gtk,
	gtk::prelude::{
		ButtonExt, CheckButtonExt, EditableExt, ListBoxRowExt, WidgetExt,
	},
	RelmWidgetExt,
};
use relm4_icons::icon_name;

use super::sub_tasks::messages::SubTaskOutput;
use super::sub_tasks::model::{SubTaskInit, SubTaskModel};

#[derive(Debug)]
pub struct TaskModel {
	pub task: Task,
	pub sub_tasks: FactoryVecDeque<SubTaskModel>,
	pub parent_list: List,
	pub index: DynamicIndex,
}

#[derive(derive_new::new)]
pub struct TaskInit {
	pub task: Task,
	pub parent_list: List,
}

#[derive(Debug)]
pub enum TaskInput {
	SetCompleted(bool),
	ModifyTitle(String),
	Favorite,
	SetNotes(Option<String>),
	SetPriority(i32),
	SetStatus(bool),
	SetToday(bool),
	SetDueDate(Option<DateTime<Utc>>),
	SetReminderDate(Option<DateTime<Utc>>),
	SetReminderHour(u32),
	SetReminderMinute(u32),
	SetDayInRecurrence((bool, Day)),
	SetDate(DateType, DateDay),
	UpdateSubTask(DynamicIndex, Task),
	RemoveSubTask(DynamicIndex),
	CreateSubTask,
}

#[derive(Debug)]
pub enum TaskOutput {
	Remove(DynamicIndex),
	UpdateTask(Task),
}

#[derive(Debug)]
pub enum DateType {
	Reminder,
	DueDate,
}

#[derive(Debug)]
pub enum DateDay {
	Today,
	Tomorrow,
	None,
}

#[relm4::factory(pub async)]
impl AsyncFactoryComponent for TaskModel {
	type ParentWidget = adw::PreferencesGroup;
	type CommandOutput = ();
	type Input = TaskInput;
	type Output = TaskOutput;
	type Init = TaskInit;
	type Widgets = TaskWidgets;

	view! {
		root = adw::ExpanderRow {
			set_expanded: false,
			#[watch]
			set_title: self.task.title.as_str(),
			#[watch]
			set_subtitle: &self.parent_list.name,
			#[name(check_button)]
			add_prefix = &gtk::CheckButton {
				set_tooltip: fl!("completed-tooltip"),
				#[watch]
				set_active: self.task.status == Status::Completed,
				connect_toggled[sender] => move |checkbox| {
					sender.input(TaskInput::SetCompleted(checkbox.is_active()));
				}
			},
			#[name(delete)]
			add_suffix = &gtk::Button {
				add_css_class: "destructive-action",
				add_css_class: "circular",
				set_icon_name: icon_name::X_CIRCULAR,
				set_tooltip: fl!("remove-task"),
				set_valign: gtk::Align::Center,
				connect_clicked[sender, index] => move |_| {
					sender.output(TaskOutput::Remove(index.clone())).unwrap()
				}
			},
			#[name(favorite)]
			add_suffix = &gtk::ToggleButton {
				add_css_class: "opaque",
				add_css_class: "circular",
				#[watch]
				set_class_active: ("favorite", self.task.favorite),
				set_icon_name: icon_name::STAR_FILLED_ROUNDED,
				set_valign: gtk::Align::Center,
				set_tooltip: fl!("favorite-task"),
				connect_clicked => TaskInput::Favorite,
			},
			add_row = &adw::EntryRow {
				set_title: fl!("title"),
				set_text: self.task.title.as_str(),
				set_show_apply_button: true,
				set_enable_emoji_completion: true,
				connect_changed[sender] => move |entry| {
					let buffer = entry.text().to_string();
					sender.input(TaskInput::ModifyTitle(buffer));
				},
				connect_activate[sender] => move |entry| {
					let buffer = entry.text().to_string();
					sender.input(TaskInput::ModifyTitle(buffer));
				},
				connect_apply[sender] => move |entry| {
					let buffer = entry.text().to_string();
					sender.input(TaskInput::ModifyTitle(buffer));
				},
			},
			add_row = &adw::EntryRow {
				set_title: fl!("notes"),
				set_show_apply_button: true,
				set_enable_emoji_completion: true,
				set_text: self.task.notes.as_deref().unwrap_or(""),
				connect_changed[sender] => move |entry| {
					let buffer = entry.text().to_string();
					if buffer.is_empty() {
						sender.input(TaskInput::SetNotes(None));
					} else {
						sender.input(TaskInput::SetNotes(Some(buffer)));
					}
				},
				connect_activate[sender] => move |entry| {
					let buffer = entry.text().to_string();
					if buffer.is_empty() {
						sender.input(TaskInput::SetNotes(None));
					} else {
						sender.input(TaskInput::SetNotes(Some(buffer)));
					}
				},
				connect_apply[sender] => move |entry| {
					let buffer = entry.text().to_string();
					if buffer.is_empty() {
						sender.input(TaskInput::SetNotes(None));
					} else {
						sender.input(TaskInput::SetNotes(Some(buffer)));
					}
				},
			},
			add_row = &adw::ActionRow {
				set_title: fl!("today"),
				set_subtitle: fl!("today-desc"),
				add_prefix = &gtk::Image {
					set_icon_name: Some(icon_name::IMAGE_ADJUST_BRIGHTNESS)
				},
				add_suffix = &gtk::Switch {
					set_tooltip: fl!("today-tooltip"),
					#[watch]
					set_active: self.task.today,
					set_valign: gtk::Align::Center,
					connect_state_set[sender] => move |_, state| {
						sender.input(TaskInput::SetToday(state));
						Propagation::Proceed
					}
				}
			},
			add_row = &adw::ActionRow {
				set_title: fl!("completed"),
				set_subtitle: fl!("completed-desc"),
				add_prefix = &gtk::Image {
					set_icon_name: Some(icon_name::CHECK_ROUND_OUTLINE_WHOLE)
				},
				add_suffix = &gtk::Switch {
					set_tooltip: fl!("completed-tooltip"),
					#[watch]
					set_active: self.task.status == Status::Completed,
					set_valign: gtk::Align::Center,
					connect_state_set[sender] => move |_, state| {
						sender.input(TaskInput::SetStatus(state));
						Propagation::Proceed
					}
				}
			},
			add_row = &adw::ActionRow {
				set_title: fl!("importance"),
				set_subtitle: fl!("importance-desc"),
				add_prefix = &gtk::Image {
					set_icon_name: Some(icon_name::WARNING)
				},
				add_suffix = &gtk::Box {
					set_css_classes: &["linked"],
					#[name(low_importance)]
					gtk::ToggleButton {
						set_icon_name: icon_name::FLAG_OUTLINE_THIN,
						set_tooltip: fl!("low"),
						set_css_classes: &["flat", "image-button"],
						set_valign: gtk::Align::Center,
						set_active: self.task.priority == Priority::Low,
						connect_toggled[sender] => move |toggle| {
							if toggle.is_active() {
								sender.input(TaskInput::SetPriority(Priority::Low as i32));
							}
						}
					},
					gtk::ToggleButton {
						set_icon_name: icon_name::FLAG_OUTLINE_THICK,
						set_tooltip: fl!("medium"),
						set_css_classes: &["flat", "image-button"],
						set_valign: gtk::Align::Center,
						set_group: Some(&low_importance),
						set_active: self.task.priority == Priority::Normal,
						connect_toggled[sender] => move |toggle| {
							if toggle.is_active() {
								sender.input(TaskInput::SetPriority(Priority::Normal as i32));
							}
						}
					},
					gtk::ToggleButton {
						set_icon_name: icon_name::FLAG_FILLED,
						set_tooltip: fl!("high"),
						set_css_classes: &["flat", "image-button"],
						set_valign: gtk::Align::Center,
						set_group: Some(&low_importance),
						set_active: self.task.priority == Priority::High,
						connect_toggled[sender] => move |toggle| {
							if toggle.is_active() {
								sender.input(TaskInput::SetPriority(Priority::High as i32));
							}
						}
					}
				}
			},
			add_row = &adw::ExpanderRow {
				set_title: fl!("reminder"),
				set_subtitle: fl!("reminder-desc"),
				set_enable_expansion: true,
				add_prefix = &gtk::Image {
					set_icon_name: Some(icon_name::ALARM)
				},
				#[name(reminder_label)]
				add_suffix = &gtk::Label {
					set_css_classes: &["accent"],
					#[watch]
					set_label: &self.task.reminder_date.map(|date| date.format("%m/%d/%Y %H:%M").to_string()).unwrap_or(fl!("no-date-set").to_string()),
					set_valign: gtk::Align::Center,
				},
				add_row = &gtk::Box {
					set_orientation: gtk::Orientation::Vertical,
					adw::ExpanderRow {
						set_title: fl!("date"),
						set_subtitle: fl!("set-date"),
						add_row = &gtk::Box {
							set_orientation: gtk::Orientation::Vertical,
							#[name(reminder_calendar)]
							gtk::Calendar {
								set_margin_all: 10,
								add_css_class: "card",
								set_day: self.task.reminder_date.unwrap_or(Utc::now()).day() as i32,
								set_month: self.task.reminder_date.unwrap_or(Utc::now()).month() as i32 - 1,
								set_year: self.task.reminder_date.unwrap_or(Utc::now()).year(),
								connect_day_selected[sender] => move |calendar| {
									if let Ok(date) = calendar.date().format("%Y-%m-%dT%H:%M:%SZ") {
										if let Ok(date) = DateTime::<Utc>::from_str(date.to_string().as_str()) {
											sender.input(TaskInput::SetReminderDate(Some(date)))
										}
									}
								}
							},
							gtk::Box {
								set_margin_all: 10,
								set_margin_bottom: 5,
								set_margin_top: 5,
								set_spacing: 10,
								gtk::Button {
									set_hexpand: true,
									set_label: fl!("today"),
									set_tooltip: fl!("set-day-today"),
									connect_clicked => TaskInput::SetDate(DateType::Reminder, DateDay::Today)
								},
								gtk::Button {
									set_hexpand: true,
									set_label: fl!("tomorrow"),
									set_tooltip: fl!("set-day-tomorrow"),
									connect_clicked => TaskInput::SetDate(DateType::Reminder, DateDay::Tomorrow)
								}
							},
							gtk::Button {
								set_margin_all:10,
								set_margin_top: 5,
								set_label: fl!("none"),
								set_tooltip: fl!("unset"),
								connect_clicked => TaskInput::SetDate(DateType::Reminder, DateDay::None)
							}
						}
					},
					adw::ExpanderRow {
						set_title: fl!("time"),
						set_subtitle: fl!("set-time"),
						add_row = &gtk::Box {
							set_css_classes: &["toolbar"],
							set_orientation: gtk::Orientation::Horizontal,
							set_halign: gtk::Align::Center,
							set_valign: gtk::Align::Center,
							set_spacing: 6,
							gtk::SpinButton {
								set_adjustment: &gtk::Adjustment::new(
									self.task.reminder_date.unwrap_or_default().time().hour() as f64, 0.0, 23.0, 1.0, 1.0, 0.0
								),
								set_orientation: gtk::Orientation::Vertical,
								set_wrap: true,
								set_numeric: true,
								set_tooltip: fl!("hour"),
								connect_value_changed[sender] => move |spin| {
									sender.input(TaskInput::SetReminderHour(spin.value() as u32))
								},
								connect_change_value[sender] => move |spin, _| {
									sender.input(TaskInput::SetReminderHour(spin.value() as u32))
								},
							},
							gtk::Label {
								set_text: ":",
							},
							gtk::SpinButton {
								set_adjustment: &gtk::Adjustment::new(
									self.task.reminder_date.unwrap_or_default().time().minute() as f64, 0.0, 59.0, 1.0, 1.0, 0.0
								),
								set_orientation: gtk::Orientation::Vertical,
								set_wrap: true,
								set_numeric: true,
								set_tooltip: fl!("minute"),
								connect_value_changed[sender] => move |spin| {
									sender.input(TaskInput::SetReminderMinute(spin.value() as u32))
								},
								connect_change_value[sender] => move |spin, _| {
									sender.input(TaskInput::SetReminderMinute(spin.value() as u32))
								},
							},
						},
					},
					adw::ExpanderRow {
						set_title: fl!("recurrence"),
						set_subtitle: fl!("set-recurrence"),
						add_row = &gtk::Box {
							set_valign: gtk::Align::Center,
							set_halign: gtk::Align::Center,
							set_margin_all: 5,
							set_css_classes: &["linked"],
							gtk::ToggleButton {
								set_label: fl!("mon"),
								set_tooltip: fl!("monday"),
								#[watch]
								set_active: self.task.recurrence.monday,
								connect_toggled[sender] => move |toggled_button| sender.input(TaskInput::SetDayInRecurrence((toggled_button.is_active(), Day::Monday)))
							},
							gtk::ToggleButton {
								set_label: fl!("tue"),
								set_tooltip: fl!("tuesday"),
								#[watch]
								set_active: self.task.recurrence.tuesday,
								connect_toggled[sender] => move |toggled_button| sender.input(TaskInput::SetDayInRecurrence((toggled_button.is_active(), Day::Tuesday)))
							},
							gtk::ToggleButton {
								set_label: fl!("wed"),
								set_tooltip: fl!("wednesday"),
								#[watch]
								set_active: self.task.recurrence.wednesday,
								connect_toggled[sender] => move |toggled_button| sender.input(TaskInput::SetDayInRecurrence((toggled_button.is_active(), Day::Wednesday)))
							},
							gtk::ToggleButton {
								set_label: fl!("thu"),
								set_tooltip: fl!("thursday"),
								#[watch]
								set_active: self.task.recurrence.thursday,
								connect_toggled[sender] => move |toggled_button| sender.input(TaskInput::SetDayInRecurrence((toggled_button.is_active(), Day::Thursday)))
							},
							gtk::ToggleButton {
								set_label: fl!("fri"),
								set_tooltip: fl!("friday"),
								#[watch]
								set_active: self.task.recurrence.friday,
								connect_toggled[sender] => move |toggled_button| sender.input(TaskInput::SetDayInRecurrence((toggled_button.is_active(), Day::Friday)))
							},
							gtk::ToggleButton {
								set_label: fl!("sat"),
								set_tooltip: fl!("saturday"),
								#[watch]
								set_active: self.task.recurrence.saturday,
								connect_toggled[sender] => move |toggled_button| sender.input(TaskInput::SetDayInRecurrence((toggled_button.is_active(), Day::Saturday)))
							},
							gtk::ToggleButton {
								set_label: fl!("sun"),
								set_tooltip: fl!("sunday"),
								#[watch]
								set_active: self.task.recurrence.sunday,
								connect_toggled[sender] => move |toggled_button| sender.input(TaskInput::SetDayInRecurrence((toggled_button.is_active(), Day::Sunday)))
							},
						},
					}
				}
			},
			add_row = &adw::ExpanderRow {
				add_prefix = &gtk::Image {
					set_icon_name: Some(icon_name::WORK_WEEK)
				},
				set_title: fl!("due-date"),
				set_subtitle: fl!("set-due-date"),
				set_enable_expansion: true,
				#[name(due_date_label)]
				add_suffix = &gtk::Label {
					set_css_classes: &["accent"],
					#[watch]
					set_label: &self.task.due_date.map(|date| date.format("%m/%d/%Y %H:%M").to_string()).unwrap_or(fl!("no-date-set").to_string()),
					set_valign: gtk::Align::Center,
				},
				add_row = &gtk::Box {
					set_orientation: gtk::Orientation::Vertical,
					#[name(due_date_calendar)]
					gtk::Calendar {
						set_margin_all: 10,
						add_css_class: "card",
						set_day: self.task.due_date.unwrap_or(Utc::now()).day() as i32,
						set_month: self.task.due_date.unwrap_or(Utc::now()).month() as i32 - 1,
						set_year: self.task.due_date.unwrap_or(Utc::now()).year(),
						connect_day_selected[sender] => move |calendar| {
							if let Ok(date) = calendar.date().format("%Y-%m-%dT%H:%M:%SZ") {
								println!("{date}");
								if let Ok(date) = DateTime::<Utc>::from_str(date.as_str()) {
										sender.input(TaskInput::SetDueDate(Some(date)))
								}
							}
						}
					},
					gtk::Box {
						set_margin_all: 10,
						set_margin_bottom: 5,
						set_margin_top: 5,
						set_spacing: 10,
						gtk::Button {
							set_hexpand: true,
							set_label: fl!("today"),
							set_tooltip: fl!("set-day-today"),
							connect_clicked[sender] => move |_| {
								sender.input(TaskInput::SetDate(DateType::DueDate, DateDay::Today));
							}
						},
						gtk::Button {
							set_hexpand: true,
							set_label: fl!("tomorrow"),
							set_tooltip: fl!("set-day-tomorrow"),
							connect_clicked[sender] => move |_| {
								sender.input(TaskInput::SetDate(DateType::DueDate, DateDay::Tomorrow));
							}
						}
					},
					gtk::Button {
						set_margin_all:10,
						set_margin_top: 5,
						set_label: fl!("none"),
						set_tooltip: fl!("unset"),
						connect_clicked[sender] => move |_| {
							sender.input(TaskInput::SetDate(DateType::DueDate, DateDay::None));
						}
					}
				}
			},
			add_row = &adw::ExpanderRow {
				#[watch]
				set_enable_expansion: !self.sub_tasks.is_empty(),
				#[watch]
				set_expanded: !self.sub_tasks.is_empty(),
				set_title: fl!("sub-tasks"),
				add_suffix = &gtk::Button {
					add_css_class: "flat",
					set_icon_name: icon_name::PLUS,
					set_tooltip: fl!("add-sub-task"),
					connect_clicked => TaskInput::CreateSubTask
				},
				#[local_ref]
				add_row = sub_tasks -> adw::PreferencesGroup {
					set_margin_all: 10,
				}
			}
		}
	}

	async fn init_model(
		init: Self::Init,
		index: &DynamicIndex,
		sender: AsyncFactorySender<Self>,
	) -> Self {
		let mut task = init.task.clone();
		task.parent = init.parent_list.id.clone();
		let mut model = Self {
			task,
			sub_tasks: FactoryVecDeque::builder()
				.launch(adw::PreferencesGroup::default())
				.forward(sender.input_sender(), |output| match output {
					SubTaskOutput::Update(index, sub_task) => {
						TaskInput::UpdateSubTask(index, sub_task)
					},
					SubTaskOutput::Remove(index) => TaskInput::RemoveSubTask(index),
				}),
			parent_list: init.parent_list,
			index: index.clone(),
		};
		{
			let mut sub_tasks_guard = model.sub_tasks.guard();
			for sub_task in init.task.sub_tasks {
				sub_tasks_guard.push_back(SubTaskInit::new(sub_task));
			}
		}
		model
	}

	fn init_widgets(
		&mut self,
		index: &DynamicIndex,
		root: Self::Root,
		_returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
		sender: AsyncFactorySender<Self>,
	) -> Self::Widgets {
		let sub_tasks = self.sub_tasks.widget();
		let widgets = view_output!();
		widgets
	}

	async fn update_with_view(
		&mut self,
		widgets: &mut Self::Widgets,
		message: Self::Input,
		sender: AsyncFactorySender<Self>,
	) {
		match message {
			TaskInput::SetNotes(notes) => {
				self.task.notes = notes;
			},
			TaskInput::SetPriority(priority) => {
				self.task.priority = priority.into();
			},
			TaskInput::SetStatus(status) => {
				self.task.status = if status {
					Status::Completed
				} else {
					Status::NotStarted
				}
			},
			TaskInput::SetToday(today) => self.task.today = today,
			TaskInput::SetCompleted(toggled) => {
				self.task.status = if toggled {
					Status::Completed
				} else {
					Status::NotStarted
				};
			},
			TaskInput::Favorite => {
				self.task.favorite = !self.task.favorite;
			},
			TaskInput::ModifyTitle(title) => {
				if title != self.task.title {
					self.task.title = title;
				}
			},
			TaskInput::SetDate(calendar, date) => {
				let date = match date {
					DateDay::Today => Some(Utc::now()),
					DateDay::Tomorrow => {
						let date =
							Utc::now().checked_add_signed(Duration::days(1)).unwrap();
						Some(date)
					},
					DateDay::None => None,
				};
				match calendar {
					DateType::Reminder => {
						sender.input(TaskInput::SetReminderDate(date));
						if let Some(date) = date {
							self.task.reminder_date = Some(date);
							widgets.reminder_calendar.set_year(date.year());
							widgets.reminder_calendar.set_month(date.month() as i32 - 1);
							widgets.reminder_calendar.set_day(date.day() as i32);
						} else {
							self.task.reminder_date = None;
						}
					},
					DateType::DueDate => {
						sender.input(TaskInput::SetDueDate(date));
						if let Some(date) = date {
							self.task.due_date = Some(date);
							widgets.due_date_calendar.set_year(date.year());
							widgets.due_date_calendar.set_month(date.month() as i32 - 1);
							widgets.due_date_calendar.set_day(date.day() as i32);
						} else {
							self.task.due_date = None;
						}
					},
				}
			},
			TaskInput::SetDueDate(due_date) => {
				if let Some(date) = due_date {
					self.task.due_date = Some(date);
				} else {
					self.task.due_date = None;
				}
			},
			TaskInput::SetReminderDate(reminder_date) => {
				if let Some(date) = reminder_date {
					self.task.reminder_date = Some(date);
				} else {
					self.task.reminder_date = None;
				}
			},
			TaskInput::SetReminderHour(hour) => {
				if let Some(date) = self.task.reminder_date {
					if let Some(new_date) = date.with_hour(hour) {
						self.task.reminder_date = Some(new_date);
					}
				} else {
					let now = Utc::now().with_hour(hour).unwrap();
					let now = now.with_minute(0).unwrap();
					self.task.reminder_date = Some(now);
				}
			},
			TaskInput::SetReminderMinute(minute) => {
				if let Some(date) = self.task.reminder_date {
					if let Some(new_date) = date.with_minute(minute) {
						self.task.reminder_date = Some(new_date);
					}
				} else {
					let now = Utc::now().with_hour(0).unwrap();
					let now = now.with_minute(minute).unwrap();
					self.task.reminder_date = Some(now);
				}
			},
			TaskInput::SetDayInRecurrence((active, day)) => match day {
				Day::Monday => self.task.recurrence.monday = active,
				Day::Tuesday => self.task.recurrence.tuesday = active,
				Day::Wednesday => self.task.recurrence.wednesday = active,
				Day::Thursday => self.task.recurrence.thursday = active,
				Day::Friday => self.task.recurrence.friday = active,
				Day::Saturday => self.task.recurrence.saturday = active,
				Day::Sunday => self.task.recurrence.sunday = active,
			},
			TaskInput::CreateSubTask => {
				let index = self.sub_tasks.guard().push_back(SubTaskInit {
					sub_task: Task::default(),
				});
				self
					.task
					.sub_tasks
					.insert(index.current_index(), Task::default());
			},
			TaskInput::UpdateSubTask(index, sub_task) => {
				self
					.task
					.sub_tasks
					.iter_mut()
					.enumerate()
					.for_each(|(i, x)| {
						if i == index.current_index() {
							*x = sub_task.clone()
						}
					});
			},
			TaskInput::RemoveSubTask(index) => {
				self.task.sub_tasks.remove(index.current_index());
				self
					.sub_tasks
					.guard()
					.remove(index.current_index())
					.unwrap();
			},
		}
		sender
			.output_sender()
			.send(TaskOutput::UpdateTask(self.task.clone()))
			.unwrap_or_default();
		self.update_view(widgets, sender);
	}
}

using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Ozi.Utilities.Common;
using Ozi.Utilities.ViewModels;

// For Key and KeyEventArgs

namespace Ozi.Utilities.Views;

public partial class FmEdit
{
    private readonly Clock _clock;
    private readonly OsClockViewModel _viewModel;

    public FmEdit(Clock clock)
    {
        InitializeComponent();

        _clock = clock; // Keep reference to original clock
        _viewModel = new OsClockViewModel(clock);
        DataContext = _viewModel;

        CbTimeZones.ItemsSource = CommonTimeZones.TimeZones2;
    }


    private void Submit(object sender, RoutedEventArgs e)
    {
        _viewModel.UpdateModel(_clock); // Update the original clock here
        DialogResult = true;
        Close();
    }


    private void Window_MouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left && e.OriginalSource as Grid != ColorPicker)
        {
            DragMove();
        }
    }

    private void FmEdit_OnLoaded(object sender, RoutedEventArgs e)
    {
        ColorPicker.Background = new SolidColorBrush(_viewModel.ClockColor);
    }

    private void ColorPicker_OnMouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        e.Handled = true;

        var dlg = new System.Windows.Forms.ColorDialog();
        if (dlg.ShowDialog() == System.Windows.Forms.DialogResult.OK)
        {
            _viewModel.ClockColor = Color.FromArgb(dlg.Color.A, dlg.Color.R, dlg.Color.G, dlg.Color.B);
            ColorPicker.Background = new SolidColorBrush(_viewModel.ClockColor);
        }
    }
}
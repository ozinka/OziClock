using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Ozi.Utilities.Helpers;
using Ozi.Utilities.ViewModels;

namespace Ozi.Utilities.Views;

public partial class Edit
{
    private readonly OsClock _osClock;
    private readonly OsClockViewModel _viewModel;

    public Edit(OsClock osClock)
    {
        InitializeComponent();

        _osClock = osClock; // Keep reference to original clock
        _viewModel = new OsClockViewModel(osClock);
        DataContext = _viewModel;

        CbTimeZones.ItemsSource = TimeZonesHelper.SystemTimeZones;
    }

    private void Submit(object sender, RoutedEventArgs e)
    {
        _viewModel.UpdateModel(_osClock);
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
            _viewModel.UpdateModel(_osClock);
        }
    }

    private void CbTimeZones_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        _viewModel.UpdateModel(_osClock); // Update the original clock here
    }

    private void TbClockName_OnTextChanged(object sender, TextChangedEventArgs e)
    {
        _viewModel.UpdateModel(_osClock);
        Console.WriteLine(TbClockName.Text);
    }

}
using System;
using System.Globalization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Shapes;
using Ozi.Utilities.Helpers;
using Ozi.Utilities.ViewModels;

namespace Ozi.Utilities.Views;

public class ColorToSolidBrushConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, CultureInfo culture)
    {
        if (value is Color color)
            return new SolidColorBrush(color);
        return DependencyProperty.UnsetValue;
    }

    public object ConvertBack(object value, Type targetType, object parameter, CultureInfo culture)
    {
        if (value is SolidColorBrush brush)
            return brush.Color;
        return DependencyProperty.UnsetValue;
    }
}

public partial class Edit
{
    private readonly OsClock _osClock;
    private readonly OsClockViewModel _viewModel;
    private readonly FmMainWindow _mainWindow;

    public Edit(OsClock osClock, FmMainWindow mainWindow)
    {
        _mainWindow = mainWindow;
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
        if (e.ChangedButton == MouseButton.Left && e.OriginalSource as Grid != ColorPickerContainer)
        {
            DragMove();
        }
    }


    private void ColorPicker_OnMouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        e.Handled = true;

        var picker = new ColorPicker
        {
            Owner = this,
            Topmost = true,
            ShowInTaskbar = false
        };

        picker.OnColorSelected = color =>
        {
            _viewModel.ClockColor = color;
            ((Border)ColorPickerContainer.Children[0]).Background = new SolidColorBrush(color);
            _viewModel.UpdateModel(_osClock);
        };

        picker.Left = this.Left + ColorPickerContainer.Margin.Left + 130;
        picker.Top = this.Top + ColorPickerContainer.Margin.Top + 110;
        picker.Show();
    }


    private void CbTimeZones_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        _viewModel.UpdateModel(_osClock); // Update the original clock here
        _mainWindow.Rulers.UpdateRulers();
    }

    private void TbClockName_OnTextChanged(object sender, TextChangedEventArgs e)
    {
        _viewModel.UpdateModel(_osClock);
        Console.WriteLine(TbClockName.Text);
    }
}
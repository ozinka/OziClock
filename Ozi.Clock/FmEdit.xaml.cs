using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Ozi.Utilities.Common;
using System.Windows.Forms;
using System.Windows.Input; // For Key and KeyEventArgs
using System.Windows;

namespace Ozi.Utilities;

public partial class FmEdit : Window
{
    private readonly OsClock _clock;
    private readonly OsClockViewModel _viewModel;

    public FmEdit(OsClock clock)
    {
        InitializeComponent();

        _clock = clock; // Keep reference to original clock
        _viewModel = new OsClockViewModel(clock);
        DataContext = _viewModel;

        cbTimeZones.ItemsSource = CommonTimeZones.TimeZones2;
    }


    private void BtnOk_Click(object sender, RoutedEventArgs e)
    {
        _viewModel.UpdateModel(_clock); // Update the original clock here
        DialogResult = true;
        Close();
    }

    
    private void Window_MouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left && e.OriginalSource is not Grid colorPickerGrid && e.OriginalSource as Grid != colorPicker) 
        { 
            DragMove(); 
        } 
    }

    private void FmEdit_OnLoaded(object sender, RoutedEventArgs e)
    {
        colorPicker.Background = new SolidColorBrush(_viewModel.ClockColor);
    }

    private void ColorPicker_OnMouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        e.Handled = true;
        
        var dlg = new System.Windows.Forms.ColorDialog();
        if (dlg.ShowDialog() == System.Windows.Forms.DialogResult.OK)
        {
            _viewModel.ClockColor = Color.FromArgb(dlg.Color.A, dlg.Color.R, dlg.Color.G, dlg.Color.B);
            colorPicker.Background = new SolidColorBrush(_viewModel.ClockColor);
        }
    }

    private void CbTimeZones_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        Console.WriteLine($"TZ: {cbTimeZones.SelectedItem}");
    }
}
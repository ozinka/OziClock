using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;

namespace osUtilite
{
    public partial class fmSettings : Window
    {
        private MainWindow fFmMain;

        public fmSettings(MainWindow fmMain)
        {
            InitializeComponent();
            this.fFmMain = fmMain;
            slTransparent.ValueChanged += slTransparent_ValueChanged;
            cbTransparency.IsChecked = fFmMain.isTransparent;
            slTransparent.IsEnabled = cbTransparency.IsChecked.Value;
            slTransparent.Value = fFmMain.Opacity;
            cbShowInTaskBar.IsChecked = fFmMain.ShowInTaskbar;
            cbShowInTaskBar.IsChecked = fFmMain.ShowInTaskbar;
            cbTopMost.IsChecked = fFmMain.Topmost;
            cbAutoHide.IsChecked = fFmMain.isAutoFold;
            cbSnap.IsChecked = fFmMain.useSnap;
        }

        private void btOk_Click(object sender, RoutedEventArgs e)
        {
            this.Close();
        }

        private void CheckBox_Checked(object sender, RoutedEventArgs e)
        {
            fFmMain.isTransparent = cbTransparency.IsChecked.Value;
            slTransparent.IsEnabled = cbTransparency.IsChecked.Value;

        }

        private void Window_Loaded(object sender, RoutedEventArgs e)
        {
            cbTransparency.IsChecked = fFmMain.isTransparent;
            slTransparent.Value = fFmMain.transparentValue;
        }

        private void Window_MouseDown(object sender, MouseButtonEventArgs e)
        {
            if (e.ChangedButton == MouseButton.Left)
                this.DragMove();
        }

        private void slTransparent_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
        {
            fFmMain.transparentValue = slTransparent.Value;
            cbTransparency.Content = "Use transparency: (" + (int)(slTransparent.Value * 100) + "%)";
        }

        private void cbShowInTaskBar_Checked(object sender, RoutedEventArgs e)
        {
            fFmMain.ShowInTaskbar = cbShowInTaskBar.IsChecked.Value;
        }

        private void cbTopMost_Checked(object sender, RoutedEventArgs e)
        {
            fFmMain.Topmost = cbTopMost.IsChecked.Value;
        }

        private void cbAutoHide_Checked(object sender, RoutedEventArgs e)
        {
            fFmMain.isAutoFold = cbAutoHide.IsChecked.Value;
        }

        private void cbSnap_Checked(object sender, RoutedEventArgs e)
        {
            fFmMain.useSnap = cbSnap.IsChecked.Value;
        }
    }
}
